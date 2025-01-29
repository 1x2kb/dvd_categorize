use database::FullMovie;
use rand::{thread_rng, Rng};

pub fn get_random(movies: &[FullMovie]) -> &FullMovie {
    let gen = thread_rng().gen_range(0..movies.len());

    &movies[gen]
}

// #[cfg(test)]
// mod tests {
//     use database::Random;

//     use super::*;

//     #[test]
//     fn selects_random() {
//         let movies: Vec<FullMovie> = (1..1000)
//             .map(|_| FullMovie::random())
//             .collect();

//         let random = get_random(&movies).clone();

//         let matched = movies
//             .iter()
//             .find(&random);

//         assert!(matched.is_some());
//         assert_eq!(
//             random,
//             matched.unwrap()
//         );
//     }
// }
