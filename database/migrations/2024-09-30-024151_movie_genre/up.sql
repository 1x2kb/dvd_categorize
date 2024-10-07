-- Your SQL goes here
create table movie_genre(
    id serial primary key,
    genre varchar(15) not null,
    movie_id int not null,

    foreign key (movie_id) references movie(id)
);

create unique index movie_genre_idx on movie_genre(movie_id, genre); 
