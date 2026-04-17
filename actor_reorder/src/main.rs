use clap::Parser;
use log::{info, warn};
use models::FullMovie;
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage},
        parameters::KeepAlive,
    },
    models::ModelOptions,
    Ollama,
};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use strsim::levenshtein;

const DEFAULT_MODEL: &str = "qwen2.5:14b";
const DEFAULT_OLLAMA_HOST: &str = "localhost";
const DEFAULT_OLLAMA_PORT: &str = "11434";
const MAX_RETRY_ATTEMPTS: usize = 4;

#[derive(Parser, Debug)]
#[command(name = "actor_reorder")]
#[command(about = "Reorder movie actors by top billing using Ollama AI", long_about = None)]
struct Args {
    /// Input CSV file path
    #[arg(value_name = "INPUT_CSV")]
    input: PathBuf,

    /// Output CSV file path (default: <input>_reordered.csv)
    #[arg(short, long, value_name = "OUTPUT_CSV")]
    output: Option<PathBuf>,

    /// Start index (0-based) of movies to process
    #[arg(short, long, default_value_t = 0)]
    start: usize,

    /// Number of movies to process (default: all remaining)
    #[arg(short, long)]
    count: Option<usize>,

    /// Dry run - process movies but don't write output file
    #[arg(short, long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let output_path = generate_output_path(&args.input, args.output.clone());
    info!("Reading movies from: {}", args.input.display());
    
    if args.dry_run {
        info!("DRY RUN MODE - only parsing CSV to identify errors");
        let _movies = read_and_parse_csv_dry_run(&args)?;
        info!("Dry run complete - CSV parsed successfully");
    } else {
        info!("Will write reordered movies to: {}", output_path.display());
        let failed_path = generate_failed_output_path(&args.input);
        info!("Will write failed movies to: {}", failed_path.display());
        
        let (ollama, model) = setup_ollama_client();
        let mut movies = read_and_parse_csv(&args, &ollama, &model).await?;
        
        // Open output files for immediate writing
        let mut success_writer = create_success_writer(&output_path)?;
        let mut failed_writer = create_failed_writer(&failed_path)?;
        
        process_movies_streaming(&mut movies, &ollama, &model, &args, &mut success_writer, &mut failed_writer).await?;
        
        info!("Finished processing all movies");
    }
    
    info!("Done!");

    Ok(())
}

async fn reorder_actors_with_retry(
    ollama: &Ollama,
    model: &str,
    movie: &FullMovie,
    csv_row: usize,
) -> Result<Vec<String>, String> {
    let mut last_error = String::new();
    
    for attempt in 1..=MAX_RETRY_ATTEMPTS {
        match reorder_actors(ollama, model, movie).await {
            Ok(actors) => {
                if attempt > 1 {
                    info!("  CSV row {} ({}): Succeeded on attempt {}", csv_row, movie.name, attempt);
                }
                return Ok(actors);
            }
            Err(e) => {
                last_error = e.clone();
                if attempt < MAX_RETRY_ATTEMPTS {
                    warn!(
                        "  CSV row {} ({}): Attempt {}/{} failed: {}. Retrying...",
                        csv_row, movie.name, attempt, MAX_RETRY_ATTEMPTS, e
                    );
                }
            }
        }
    }
    
    Err(format!(
        "Failed after {} attempts. Last error: {}",
        MAX_RETRY_ATTEMPTS, last_error
    ))
}

async fn reorder_actors(
    ollama: &Ollama,
    model: &str,
    movie: &FullMovie,
) -> Result<Vec<String>, String> {
    let actor_list = movie
        .actors
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let system_prompt = r#"You are a movie expert assistant. Your task is to reorder a list of actors for a given movie based on their BILLING ORDER (the order they appear in the opening/closing credits), NOT their popularity or fame.

!!!!! CRITICAL RULES - FOLLOW EXACTLY OR YOUR RESPONSE WILL BE REJECTED !!!!!

1. ABSOLUTELY NEVER ADD ANY ACTORS NOT IN THE PROVIDED LIST
2. ABSOLUTELY NEVER REMOVE ANY ACTORS FROM THE PROVIDED LIST
3. Output array MUST contain THE EXACT SAME ACTORS as input - NO MORE, NO LESS
4. Output array MUST have the EXACT SAME COUNT as input list
5. Your ONLY job is to REORDER the existing actors - DO NOT ADD OR REMOVE ANYONE
6. You may fix minor spelling errors in actor names (e.g., "Kurt Russel" -> "Kurt Russell")
7. CONVERT ALL ACCENTED CHARACTERS TO ENGLISH EQUIVALENTS (e.g., "Renée" -> "Renee", "José" -> "Jose", "Zoë" -> "Zoe")
8. USE ONLY STANDARD ENGLISH LETTERS (a-z, A-Z) - NO ACCENTED CHARACTERS ALLOWED

DO NOT ADD ACTORS. DO NOT REMOVE ACTORS. ONLY REORDER THE EXACT LIST PROVIDED.

Billing order reflects each actor's role size in THIS SPECIFIC MOVIE, not their overall career fame.

Example: Ryan Reynolds appears in "Bullet Train" but has a very small cameo role, so he should be near the END of the billing order despite being more famous than the leads.

Ordering Guidelines:
- Lead roles (most screen time, central to plot) come first
- Supporting roles come in the middle
- Cameos and minor roles come last - even if the actor is very famous
- Base ordering on the SIZE and IMPORTANCE of each actor's role in THIS SPECIFIC MOVIE
- If you're unsure, make your best educated guess based on the movie title, year, and description

REMINDER: Use ONLY the actors provided in the input list. DO NOT ADD any actors even if you think they should be included.

TWO-STEP PROCESS:
Step 1: Reorder the actors by billing importance
Step 2: BEFORE returning your answer, verify your reordered list has the EXACT SAME COUNT as the input
        - Count the input actors
        - Count your output actors
        - If counts don't match, fix your list to match the input count exactly
        - Only return your answer after verifying the counts match

Output Format:
- Return ONLY a JSON array: ["Actor Name 1", "Actor Name 2", "Actor Name 3"]
- The array must contain EXACTLY the same actors as provided, just reordered
- Do not include any explanation or additional text - ONLY the JSON array"#;

    let movie_info = format!(
        "Movie Title: {}\nRelease Year: {} (IMPORTANT: Use this year to identify the correct version of the movie)\n\nActors to reorder (these actors ARE in the {} {} version): {}",
        movie.name,
        movie.release_year,
        movie.release_year,
        movie.name,
        actor_list
    );

    let messages = vec![
        ChatMessage::system(system_prompt.to_string()),
        ChatMessage::user(movie_info),
    ];

    let request = ChatMessageRequest::new(model.to_string(), messages)
        .options(
            ModelOptions::default()
                .num_ctx(32768)
                .temperature(0.3)
        );

    let response = ollama
        .send_chat_messages(request)
        .await
        .map_err(|e| format!("Ollama error: {}", e))?;

    let content = response.message.content.trim();

    let json_content = extract_json_array(content);

    let reordered: Vec<String> = serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse JSON response: {}. Response was: {}", e, content))?;
    
    // Validate only standard English characters - reject accented chars and Chinese
    for actor in &reordered {
        if actor.chars().any(|c| !c.is_ascii_alphanumeric() && !matches!(c, ' ' | '.' | '-' | '\'' | ',' | '"')) {
            return Err(format!(
                "Actor name contains non-English characters: '{}'",
                actor
            ));
        }
    }
    
    // Validate response - must match exact count
    let original_count = movie.actors.len();
    if reordered.len() != original_count {
        return Err(format!(
            "AI returned {} actors but expected exactly {}. Ignoring response.",
            reordered.len(),
            original_count
        ));
    }
    
    // Validate all returned actors exist in original list using fuzzy matching
    // Allows minor spelling fixes (e.g., "Kurt Russel" -> "Kurt Russell")
    // Max Levenshtein distance of 2 allows single char add/remove/change
    const MAX_EDIT_DISTANCE: usize = 2;
    
    for actor in &reordered {
        let mut found_match = false;
        for original in &movie.actors {
            let distance = levenshtein(&actor.to_lowercase(), &original.name.to_lowercase());
            if distance <= MAX_EDIT_DISTANCE {
                found_match = true;
                break;
            }
        }
        if !found_match {
            return Err(format!(
                "AI returned actor '{}' not matching any original actor (max edit distance {}). Ignoring response.",
                actor, MAX_EDIT_DISTANCE
            ));
        }
    }
    
    Ok(reordered)
}

fn extract_json_array(s: &str) -> &str {
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            if end >= start {
                return &s[start..=end];
            }
        }
    }
    s
}

fn read_and_parse_csv_dry_run(args: &Args) -> Result<Vec<FullMovie>, Box<dyn std::error::Error>> {
    // Count CSV records properly (handles multiline quoted fields)
    let file = File::open(&args.input)?;
    let mut rdr = csv::Reader::from_reader(file);
    let total_lines = rdr.records().count();
    info!("Total movie records in CSV: {}", total_lines);
    
    // Read header for display
    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let header = lines
        .next()
        .ok_or("CSV file is empty")?
        .map_err(|e| format!("Failed to read header: {}", e))?;
    
    info!("CSV header: {}", header);

    if args.start >= total_lines {
        return Err(format!(
            "Start index {} is beyond the total number of movies ({})",
            args.start, total_lines
        )
        .into());
    }

    let end_index = if let Some(count) = args.count {
        (args.start + count).min(total_lines)
    } else {
        total_lines
    };

    let movies_to_process = end_index - args.start;
    info!(
        "Will process movies from index {} to {} ({} movies)",
        args.start,
        end_index - 1,
        movies_to_process
    );

    let csv_bytes = read_csv_range(&args.input, &header, args.start, movies_to_process)?;
    
    info!("Parsing selected CSV records (dry run)...");
    let movies = match csv_utils::parse_csv(csv_bytes.as_slice()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n=== CSV Parsing Error ===");
            eprintln!("Error: {}", e);
            
            eprintln!("\n=== CSV Parsing Failed ===");
            eprintln!("Unable to parse CSV content");
            eprintln!("\n=== End ===\n");
            
            return Err(e);
        }
    };
    info!("Successfully parsed {} movie records", movies.len());
    
    Ok(movies)
}

fn generate_output_path(input_path: &PathBuf, output: Option<PathBuf>) -> PathBuf {
    output.unwrap_or_else(|| input_path.clone())
}

fn generate_failed_output_path(input_path: &PathBuf) -> PathBuf {
    let mut failed_path = input_path.clone();
    let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = input_path.extension().unwrap_or_default().to_string_lossy();
    let new_name = format!("{}-failed.{}", stem, extension);
    failed_path.set_file_name(new_name);
    failed_path
}

fn create_success_writer(output_path: &PathBuf) -> Result<csv::Writer<File>, Box<dyn std::error::Error>> {
    let file = File::create(output_path)?;
    let writer = csv::Writer::from_writer(file);
    Ok(writer)
}

fn create_failed_writer(output_path: &PathBuf) -> Result<File, Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut file = File::create(output_path)?;
    // Write header
    writeln!(file, "Title,Year,Description,Actors,Genres,Director,AddedOn,Location,Error")?;
    Ok(file)
}

fn write_movie_to_csv(writer: &mut csv::Writer<File>, movie: &FullMovie) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Serialize;
    
    #[derive(Serialize)]
    struct CsvMovie {
        #[serde(rename = "Title")]
        title: String,
        #[serde(rename = "Year")]
        year: i32,
        #[serde(rename = "Description")]
        description: String,
        #[serde(rename = "Actors")]
        actors: String,
        #[serde(rename = "Genres")]
        genres: String,
        #[serde(rename = "Director")]
        director: String,
        #[serde(rename = "AddedOn")]
        added_on: String,
        #[serde(rename = "Location")]
        location: String,
    }
    
    let csv_movie = CsvMovie {
        title: movie.name.clone(),
        year: movie.release_year,
        description: movie.description.as_deref().unwrap_or("").to_string(),
        actors: movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(" | "),
        genres: movie.genres.join(" | "),
        director: movie.director.as_ref().map(|d| d.name.as_str()).unwrap_or("").to_string(),
        added_on: movie.added_on.as_deref().unwrap_or("").to_string(),
        location: movie.location.as_deref().unwrap_or("").to_string(),
    };
    
    writer.serialize(csv_movie)?;
    writer.flush()?;
    Ok(())
}

fn write_failed_movie(file: &mut File, movie: &FullMovie, error: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    
    let actors = movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join("; ");
    let genres = movie.genres.join("; ");
    let director = movie.director.as_ref().map(|d| d.name.as_str()).unwrap_or("");
    let description = movie.description.as_deref().unwrap_or("");
    let added_on = movie.added_on.as_deref().unwrap_or("");
    let location = movie.location.as_deref().unwrap_or("");
    
    // Escape fields that might contain commas or quotes
    let escape = |s: &str| {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"" , s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    
    writeln!(
        file,
        "{},{},{},{},{},{},{},{},{}",
        escape(&movie.name),
        movie.release_year,
        escape(description),
        escape(&actors),
        escape(&genres),
        escape(director),
        escape(added_on),
        escape(location),
        escape(error)
    )?;
    file.flush()?;
    Ok(())
}

async fn read_and_parse_csv(args: &Args, ollama: &Ollama, model: &str) -> Result<Vec<FullMovie>, Box<dyn std::error::Error>> {
    // Count CSV records properly (handles multiline quoted fields)
    let file = File::open(&args.input)?;
    let mut rdr = csv::Reader::from_reader(file);
    let total_lines = rdr.records().count();
    info!("Total movie records in CSV: {}", total_lines);
    
    // Read header for display
    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let header = lines
        .next()
        .ok_or("CSV file is empty")?
        .map_err(|e| format!("Failed to read header: {}", e))?;
    
    info!("CSV header: {}", header);

    if args.start >= total_lines {
        return Err(format!(
            "Start index {} is beyond the total number of movies ({})",
            args.start, total_lines
        )
        .into());
    }

    let end_index = if let Some(count) = args.count {
        (args.start + count).min(total_lines)
    } else {
        total_lines
    };

    let movies_to_process = end_index - args.start;
    info!(
        "Will process movies from index {} to {} ({} movies)",
        args.start,
        end_index - 1,
        movies_to_process
    );

    let csv_bytes = read_csv_range(&args.input, &header, args.start, movies_to_process)?;
    
    info!("Parsing selected CSV records...");
    let movies = match csv_utils::parse_csv(csv_bytes.as_slice()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n=== CSV Parsing Error ===");
            eprintln!("Error: {}", e);
            
            eprintln!("\n=== CSV Parsing Failed ===");
            eprintln!("Unable to parse CSV content");
            eprintln!("\n=== End ===\n");
            
            return Err(e);
        }
    };
    info!("Successfully parsed {} movie records", movies.len());
    
    Ok(movies)
}

fn read_csv_range(
    input_path: &PathBuf,
    _header: &str,
    start: usize,
    count: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Use CSV parser to properly handle multiline quoted fields
    let file = File::open(input_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    
    // Get headers
    let headers = rdr.headers()?.clone();
    
    // Skip to start index
    let mut records = rdr.records();
    for _ in 0..start {
        records.next();
    }
    
    // Collect requested records
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers)?;
    
    for _ in 0..count {
        if let Some(result) = records.next() {
            let record = result?;
            wtr.write_record(&record)?;
        } else {
            break;
        }
    }
    
    Ok(wtr.into_inner()?)
}

async fn unload_model(ollama: &Ollama, model: &str) -> Result<(), String> {
    use ollama_rs::generation::completion::request::GenerationRequest;
    
    let mut request = GenerationRequest::new(model.to_string(), String::new());
    request.keep_alive = Some(KeepAlive::UnloadOnCompletion);
    
    ollama
        .generate(request)
        .await
        .map_err(|e| format!("Failed to unload model: {}", e))?;
    
    Ok(())
}

fn setup_ollama_client() -> (Ollama, String) {
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port = env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let ollama_url = format!("http://{}:{}", &ollama_host, &ollama_port);
    info!("Connecting to Ollama at {}", &ollama_url);

    let ollama = Ollama::from_url(ollama_url.parse().expect("Invalid Ollama URL"));
    (ollama, model)
}

async fn process_movies_streaming(
    movies: &mut [FullMovie],
    ollama: &Ollama,
    model: &str,
    args: &Args,
    success_writer: &mut csv::Writer<File>,
    failed_writer: &mut File,
) -> Result<(), Box<dyn std::error::Error>> {
    let movies_to_process = movies.len();
    let reset_model_count = 50;
    
    for (idx, movie) in movies.iter_mut().enumerate() {
        // Unload model every 50 movies to prevent state buildup
        if idx > 0 && idx % reset_model_count == 0 {
            info!("Unloading model after {} movies to prevent state buildup", idx);
            if let Err(e) = unload_model(ollama, model).await {
                warn!("Failed to unload model: {}", e);
            }
        }
        let absolute_idx = args.start + idx;
        let csv_row = absolute_idx + 2;
        
        if movie.actors.is_empty() {
            info!(
                "[{}/{}] CSV row {} ({}): Skipping - no actors listed",
                idx + 1,
                movies_to_process,
                csv_row,
                movie.name
            );
            // Write skipped movie to success file with original data
            write_movie_to_csv(success_writer, movie)?;
            continue;
        }

        info!(
            "[{}/{}] CSV row {} ({}): Processing {} actors",
            idx + 1,
            movies_to_process,
            csv_row,
            movie.name,
            movie.actors.len()
        );

        match reorder_actors_with_retry(ollama, model, movie, csv_row).await {
            Ok(reordered_actors) => {
                log_actor_reordering(movie, &reordered_actors, csv_row);
                update_movie_actors(movie, reordered_actors, csv_row);
                // Write successful movie immediately
                write_movie_to_csv(success_writer, movie)?;
            }
            Err(e) => {
                warn!("  CSV row {} ({}): Failed to reorder actors: {}", csv_row, movie.name, e);
                warn!("  CSV row {} ({}): Keeping original actor order", csv_row, movie.name);
                // Write failed movie immediately
                write_failed_movie(failed_writer, movie, &e)?;
                // Also write to success file with original order
                write_movie_to_csv(success_writer, movie)?;
            }
        }
    }
    
    Ok(())
}

fn log_actor_reordering(movie: &FullMovie, reordered_actors: &[String], csv_row: usize) {
    info!(
        "  CSV row {} ({}): Original actors: {}",
        csv_row,
        movie.name,
        movie
            .actors
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    info!(
        "  CSV row {} ({}): Reordered actors: {}",
        csv_row,
        movie.name,
        reordered_actors.join(" | ")
    );
}

fn update_movie_actors(movie: &mut FullMovie, reordered_actors: Vec<String>, csv_row: usize) {
    movie.actors = reordered_actors
        .into_iter()
        .map(|name| models::Actor::from(name))
        .collect();
    info!("  CSV row {} ({}): Successfully reordered actors", csv_row, movie.name);
}


fn extract_error_line(error_msg: &str, csv_content: &str) -> Option<(usize, String)> {
    let line_num = if error_msg.contains("line: ") {
        error_msg
            .split("line: ")
            .nth(1)?
            .split(',')
            .next()?
            .trim()
            .parse::<usize>()
            .ok()?
    } else {
        return None;
    };
    
    let lines: Vec<&str> = csv_content.lines().collect();
    if line_num > 0 && line_num <= lines.len() {
        Some((line_num, lines[line_num - 1].to_string()))
    } else {
        None
    }
}

async fn debug_csv_error_with_ai(
    ollama: &Ollama,
    model: &str,
    error_msg: &str,
    problematic_line: &str,
    line_number: usize,
) -> Result<String, String> {
    let system_prompt = r#"You are a CSV parsing debugging expert. Analyze the error and the problematic CSV line to identify the root cause and suggest fixes.

Provide:
1. Root cause of the parsing error
2. Which field(s) are causing the issue
3. Concrete fix suggestions for this specific line

IMPORTANT! The CSV should have exactly 8 fields per row:
- Title
- Year
- Description
- Actors
- Genres
- Director
- AddedOn
- Location

Do not provide code solutions. Just explain the issue with this specific line.

Be VERY concise and actionable."#;

    let user_prompt = format!(
        "CSV Parsing Error:\n{}\n\nProblematic Line {} Content:\n{}\n\nWhat's wrong with this line and how to fix it?",
        error_msg,
        line_number,
        problematic_line
    );

    let messages = vec![
        ChatMessage::system(system_prompt.to_string()),
        ChatMessage::user(user_prompt),
    ];

    let request = ChatMessageRequest::new(model.to_string(), messages)
        .options(
            ModelOptions::default()
                .num_ctx(32768)
                .temperature(0.3)
        );

    let response = ollama
        .send_chat_messages(request)
        .await
        .map_err(|e| format!("Ollama error: {}", e))?;

    Ok(response.message.content.trim().to_string())
}
