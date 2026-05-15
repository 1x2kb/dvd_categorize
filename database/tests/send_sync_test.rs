// Test that our repository types are Send + Sync
use database::postgres::*;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_movie_repository_is_send_sync() {
    assert_send::<PostgresMovieRepository>();
    assert_sync::<PostgresMovieRepository>();
    assert_send_sync::<PostgresMovieRepository>();
}

#[test]
fn test_actor_repository_is_send_sync() {
    assert_send::<PostgresActorRepository>();
    assert_sync::<PostgresActorRepository>();
    assert_send_sync::<PostgresActorRepository>();
}

#[test]
fn test_director_repository_is_send_sync() {
    assert_send::<PostgresDirectorRepository>();
    assert_sync::<PostgresDirectorRepository>();
    assert_send_sync::<PostgresDirectorRepository>();
}

#[test]
fn test_genre_repository_is_send_sync() {
    assert_send::<PostgresGenreRepository>();
    assert_sync::<PostgresGenreRepository>();
    assert_send_sync::<PostgresGenreRepository>();
}
