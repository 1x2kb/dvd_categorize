use std::fmt::Display;

use serde::Deserialize;

pub const DATA_POINTS_PROMPT: &'static str = r#"
Analyze the user's movie-related question and extract filter criteria matching this database schema:

**Target Filters to Identify:**
- `actors`: Names of performers/voices mentioned
- `directors`: Names of filmmakers referenced
- `genres`: Movie categories specified (15 character max)
- `plot_keywords` - Descriptive terms in plots

**Response Requirements:**
1. Output JSON with *only* specified filter types present in query
2. Use exact noun phrases from the question
3. Support multi-criteria searches through array values
4. Do not add any additional context to the users question. If the user did not specify a movie name, don't include it in the JSON. If the user did not include any genres, do not include it in the JSON.
5. IMPORTANT!: Use only json in your response. Do not include any words outside of the requested JSON. The response is only to include the JSON response. Do not provide any markdown or rich text. Just {*}
7. Make sure that you ONLY included in the JSON response what the USER asked for. Make sure you did not add any actors, directors, and/or any movies the user did not ask for.

**Examples:**
Q: "Comedy movies by Christopher Nolan"
A: {"genres": ["Comedy"], "directors": ["Christopher Nolan"]}

Q: "Sci-fi films with Tom Cruise"
A: {"genres": ["Sci-fi"], "actors": ["Tom Cruise"]}

Q: "Dramas about war with Meryl Streep"
A: {"genres": ["Drama"], "actors": ["Meryl Streep"], "plot_keywords": ["war"]}

Q: "Suggest an action movie that is also a drama"
A: {"genres": ["Action", "Drama"]}

Q: "Suggest a romance movie"
A: {"genres": ["Romance"]}

**Important!**
- You are not to answer the users question. You are to only analyze the question and find the relevant data points. Another AI will answer the question. You are helping them to answer the question.
"#;
