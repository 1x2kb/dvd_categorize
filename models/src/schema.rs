// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    actor (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    director (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    movie (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        director_id -> Nullable<Int4>,
        description -> Nullable<Text>,
        embedding -> Nullable<Vector>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    movie_actor (id) {
        id -> Int4,
        movie_id -> Int4,
        actor_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    movie_genre (id) {
        id -> Int4,
        movie_id -> Int4,
        #[max_length = 30]
        genre -> Varchar,
    }
}

diesel::joinable!(movie -> director (director_id));
diesel::joinable!(movie_actor -> actor (actor_id));
diesel::joinable!(movie_actor -> movie (movie_id));
diesel::joinable!(movie_genre -> movie (movie_id));

diesel::allow_tables_to_appear_in_same_query!(
    actor,
    director,
    movie,
    movie_actor,
    movie_genre,
);
