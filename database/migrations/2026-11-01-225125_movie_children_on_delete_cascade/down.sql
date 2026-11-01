-- This file should undo anything in `up.sql`
alter table movie_actor drop constraint movie_actor_movie_id_fkey;
alter table movie_actor add constraint movie_actor_movie_id_fkey
    foreign key (movie_id) references movie (id);

alter table movie_genre drop constraint movie_genre_movie_id_fkey;
alter table movie_genre add constraint movie_genre_movie_id_fkey
    foreign key (movie_id) references movie (id);