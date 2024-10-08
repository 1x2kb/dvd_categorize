// @generated automatically by Diesel CLI.

diesel::table! {
    actor (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
    }
}

diesel::table! {
    director (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
    }
}

diesel::table! {
    movie (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        director_id -> Nullable<Int4>,
        description -> Nullable<Text>,
    }
}

diesel::table! {
    movie_actor (id) {
        id -> Int4,
        movie_id -> Int4,
        actor_id -> Int4,
    }
}

diesel::table! {
    movie_genre (id) {
        id -> Int4,
        #[max_length = 15]
        genre -> Varchar,
        movie_id -> Int4,
    }
}

diesel::joinable!(movie -> director (director_id));
diesel::joinable!(movie_actor -> actor (actor_id));
diesel::joinable!(movie_actor -> movie (movie_id));
diesel::joinable!(movie_genre -> movie (movie_id));

diesel::allow_tables_to_appear_in_same_query!(actor, director, movie, movie_actor, movie_genre,);
