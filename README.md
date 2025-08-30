# Dvd Categorize

## Setup
- Install docker and docker compose (if not packaged with docker for your system)
- `docker volume create ollama_data` - Create ollama_data docker volume for ollama to store models. This is done manually so docker compose down -v does not delete the large model files.
- Models can be found here: https://ollama.com/search. You'll want `phi3.5`, `llama3.2`, and `nomic-embed-text` (unless changed)
- Run `sudo docker compose -f docker-compose-prod.yml up` to build and run the containers. # these are build with --release and make take awhile to build but should only need to be built once.
- Copy test.csv to movies.csv
    - Run `sudo docker-compose -f docker-compose-prod.yml --profile database_load up csv_reader` to load movies.csv into the database. 
- Default app is available at localhost:8080

## Local Development
- Run `sudo docker-compose up` to build and run the containers. # these are built by binding the local volume so local builds are still possible and even beneficial.
- Add data from movies.csv, `docker compose --profile tools run --rm csv_reader`
- Default app is available at localhost:8080
