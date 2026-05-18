Added tools for ai functions to let ollama call a function to get data on the user query.

I tried a few things with diesel-async to get the function to work but ollama-rs requires the code to be Send + Sync. Diesel-async however is only Send. Tried with a syncronous implementation and same result it's not Sync.

spawn_blocking did not work because it still realized the code that powered the database was not sync. Even thoguh we were blocking it's a hard requirement from ollama-rs.

Potential switch to Mongodb + Redis for vector search, which has code partially completed in another branch. Or just implement a rag solution involving multiple models or AI calls, one to collect relevant data. One to read the results and answer the query.