/**
 * Queries the local Ollama instance for pulled models and returns the name of
 * the smallest one by disk size. Useful for picking a fast model in AI tests.
 *
 * Ollama is exposed on host port 11434 (docker-compose: 0.0.0.0:11434:11434).
 */

const OLLAMA_URL = process.env.OLLAMA_URL ?? 'http://localhost:11434';

interface OllamaModel {
  name: string;
  size: number;
}

interface OllamaTagsResponse {
  models: OllamaModel[];
}

/**
 * Returns the name of the smallest pulled Ollama model by byte size.
 * Falls back to `phi3.5` if the API is unreachable or returns no models.
 */
export async function smallestOllamaModel(): Promise<string> {
  try {
    const res = await fetch(`${OLLAMA_URL}/api/tags`);
    if (!res.ok) return 'phi3.5';
    const data: OllamaTagsResponse = await res.json();
    if (!data.models?.length) return 'phi3.5';
    const sorted = [...data.models].sort((a, b) => a.size - b.size);
    return sorted[0].name;
  } catch {
    return 'phi3.5';
  }
}
