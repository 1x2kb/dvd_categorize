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
    pub data: Vec<(String, f64)>,
}
