use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsOverview {
    pub total_movies: usize,
    pub total_directors: usize,
    pub total_actors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarChartData {
    pub labels: Vec<String>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieChartData {
    pub data: Vec<(
        String,
        f64,
    )>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomSelectionStats {
    pub total_movies: usize,
    pub genre_count: usize,
    pub actor_count: usize,
    pub genres: Vec<RandomSelectionStat>,
    pub actors: Vec<RandomSelectionStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomSelectionStat {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomSelectionOddsResponse {
    pub total_movies: usize,
    pub random_count: usize,
    pub genres: Vec<RandomOddsItem>,
    pub actors: Vec<RandomOddsItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomOddsItem {
    pub label: String,
    pub count: usize,
    pub per_movie_probability: f64,
    pub per_movie_odds: f64,
    pub at_least_one_probability: f64,
}
