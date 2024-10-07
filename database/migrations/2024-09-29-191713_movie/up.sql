-- Your SQL goes here
create table movie(
    id serial primary key,
    name varchar(100) not null,
    director_id int,

    foreign key (director_id) references director(id)
);
 
 create index movie_director_idx on movie (director_id);
 