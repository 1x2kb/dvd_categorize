use clap::Parser;
use log::{info, warn};
use models::FullMovie;
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

const DEFAULT_MODEL: &str = "phi3.5";
const DEFAULT_OLLAMA_HOST: &str = "localhost";
const DEFAULT_OLLAMA_PORT: &str = "11434";
const MAX_RETRY_ATTEMPTS: usize = 3;

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
        let (ollama, model) = setup_ollama_client();
        let mut movies = read_and_parse_csv(&args, &ollama, &model).await?;
        process_movies(&mut movies, &ollama, &model, &args).await;
        write_output(&output_path, &movies)?;
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

    let system_prompt = r#"You are a movie expert assistant. Your task is to reorder a list of actors for a given movie from top-billed (most important/lead roles) to least-billed (supporting roles).

Rules:
1. Return ONLY a JSON array of actor names in the correct billing order
2. Do not add or remove any actors - use exactly the names provided
3. Base your ordering on your knowledge of the movie and typical billing practices
4. Lead actors come first, supporting actors last
5. If you're unsure, make your best educated guess based on the movie title, year, and description
6. Format: ["Actor Name 1", "Actor Name 2", "Actor Name 3"]
7. Do not include any explanation or additional text - ONLY the JSON array"#;

    let movie_info = format!(
        "Movie: {} ({})\nDescription: {}\nActors to reorder: {}",
        movie.name,
        movie.release_year,
        movie.description.as_deref().unwrap_or("No description"),
        actor_list
    );

    let messages = vec![
        ChatMessage::system(system_prompt.to_string()),
        ChatMessage::user(movie_info),
    ];

    let request = ChatMessageRequest::new(model.to_string(), messages);

    let response = ollama
        .send_chat_messages(request)
        .await
        .map_err(|e| format!("Ollama error: {}", e))?;

    let content = response.message.content.trim();

    let json_content = extract_json_array(content);

    serde_json::from_str::<Vec<String>>(json_content)
        .map_err(|e| format!("Failed to parse JSON response: {}. Response was: {}", e, content))
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
    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header = lines
        .next()
        .ok_or("CSV file is empty")?
        .map_err(|e| format!("Failed to read header: {}", e))?;
    
    info!("CSV header: {}", header);

    let total_lines: usize = lines.count();
    info!("Total movie records in CSV: {}", total_lines);

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

    let selected_lines = read_csv_range(&args.input, &header, args.start, movies_to_process)?;
    
    info!("Parsing {} selected CSV lines...", selected_lines.len() - 1);
    let csv_content = selected_lines.join("\n");
    let movies = match csv_utils::parse_csv(csv_content.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n=== CSV Parsing Error ===");
            eprintln!("Error: {}", e);
            
            let error_line = extract_error_line(&e.to_string(), &csv_content);
            if let Some((line_num, line_content)) = error_line {
                eprintln!("\n=== Problematic CSV Line {} ===", line_num);
                eprintln!("{}", line_content);
                eprintln!("\n=== End Line ===\n");
            } else {
                eprintln!("\n=== Raw CSV Content ===");
                eprintln!("{}", csv_content);
                eprintln!("\n=== End Raw Content ===\n");
            }
            
            return Err(e);
        }
    };
    info!("Successfully parsed {} movie records", movies.len());
    
    Ok(movies)
}

fn generate_output_path(input_path: &PathBuf, output: Option<PathBuf>) -> PathBBuf {
    output.unwrap_or_else(|| {
        let mut path = input_path.clone();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("csv");
        path.set_file_name(format!("{}_reordered.{}", stem, ext));
        path
    })
}

async fn read_and_parse_csv(args: &Args, ollama: &Ollama, model: &str) -> Result<Vec<FullMovie>, Box<dyn std::error::Error>> {
    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header = lines
        .next()
        .ok_or("CSV file is empty")?
        .map_err(|e| format!("Failed to read header: {}", e))?;
    
    info!("CSV header: {}", header);

    let total_lines: usize = lines.count();
    info!("Total movie records in CSV: {}", total_lines);

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

    let selected_lines = read_csv_range(&args.input, &header, args.start, movies_to_process)?;
    
    info!("Parsing {} selected CSV lines...", selected_lines.len() - 1);
    let csv_content = selected_lines.join("\n");
    let movies = match csv_utils::parse_csv(csv_content.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n=== CSV Parsing Error ===");
            eprintln!("Error: {}", e);
            
            let error_line = extract_error_line(&e.to_string(), &csv_content);
            if let Some((line_num, line_content)) = error_line {
                eprintln!("\n=== Problematic CSV Line {} ===", line_num);
                eprintln!("{}", line_content);
                eprintln!("\n=== End Line ===\n");
                
                eprintln!("\n=== AI Debugging Analysis ===");
                match debug_csv_error_with_ai(ollama, model, &e.to_string(), &line_content, line_num).await {
                    Ok(analysis) => {
                        eprintln!("{}", analysis);
                        eprintln!("\n=== End AI Analysis ===\n");
                    }
                    Err(ai_err) => {
                        eprintln!("Failed to get AI analysis: {}", ai_err);
                    }
                }
            } else {
                eprintln!("\n=== Raw CSV Content ===");
                eprintln!("{}", csv_content);
                eprintln!("\n=== End Raw Content ===\n");
            }
            
            return Err(e);
        }
    };
    info!("Successfully parsed {} movie records", movies.len());
    
    Ok(movies)
}

fn read_csv_range(
    input_path: &PathBuf,
    header: &str,
    start: usize,
    count: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut lines_iter = reader.lines();

    lines_iter.next();

    for _ in 0..start {
        lines_iter.next();
    }

    let mut selected_lines = Vec::new();
    selected_lines.push(header.to_string());
    
    for _ in 0..count {
        if let Some(Ok(line)) = lines_iter.next() {
            selected_lines.push(line);
        } else {
            break;
        }
    }
    
    Ok(selected_lines)
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

async fn process_movies(
    movies: &mut [FullMovie],
    ollama: &Ollama,
    model: &str,
    args: &Args,
) {
    let movies_to_process = movies.len();
    
    for (idx, movie) in movies.iter_mut().enumerate() {
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
            }
            Err(e) => {
                warn!("  CSV row {} ({}): Failed to reorder actors: {}", csv_row, movie.name, e);
                warn!("  CSV row {} ({}): Keeping original actor order", csv_row, movie.name);
            }
        }
    }
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

fn write_output(output_path: &PathBuf, movies: &[FullMovie]) -> Result<(), Box<dyn std::error::Error>> {
    info!("Writing reordered movies to: {}", output_path.display());
    let csv_content = csv_utils::movies_to_csv(movies)?;
    std::fs::write(output_path, csv_content)?;
    info!("Successfully wrote {} movies to: {}", movies.len(), output_path.display());
    Ok(())
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

    let request = ChatMessageRequest::new(model.to_string(), messages);

    let response = ollama
        .send_chat_messages(request)
        .await
        .map_err(|e| format!("Ollama error: {}", e))?;

    Ok(response.message.content.trim().to_string())
}
