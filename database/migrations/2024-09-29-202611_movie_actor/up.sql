-- Your SQL goes here
create table movie_actor(
    id serial primary key,
    movie_id int not null,
    actor_id int not null,

    foreign key (movie_id) references movie(id),
    foreign key (actor_id) references actor(id)
);

create unique index movie_actor_id_idx on movie_actor(movie_id, actor_id);
