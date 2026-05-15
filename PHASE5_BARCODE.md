# Phase 5: Barcode Scanning for DVD Entry

## Status: Ready after Phase 4 complete

## Goal
Scan DVD barcode (UPC/EAN) → auto-fetch movie info → one-click add to library.

## Current State
Manual CSV entry = slow, error-prone.

## Solution
Camera barcode scan → lookup APIs → auto-populate fields.

## Architecture

```
[Phone Camera] → [Barcode Scanner] → [UPC Lookup API] → [Movie DB API] → [Auto-fill Form]
```

## Dependencies

### Backend
```toml
# dvd_catalog_api/Cargo.toml
reqwest = { version = "0.12", features = ["json"] }
```

### Frontend (Dioxus Web)
```toml
# dvd_categorizer_web/Cargo.toml
[dependencies]
web-sys = { version = "0.3", features = [
    "MediaDevices",
    "MediaStream",
    "MediaStreamConstraints",
    "Navigator",
    "VideoTrack",
    "Window",
] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
```

### JavaScript Barcode Library
Add to `index.html`:
```html
<script src="https://unpkg.com/@zxing/library@latest"></script>
```

## External APIs

### 1. UPC Lookup (Free)
- **API:** UPCitemdb.com (free tier: 100 req/day)
- **Endpoint:** `https://api.upcitemdb.com/prod/trial/lookup?upc={barcode}`
- **Returns:** Product name, brand, category

### 2. Movie Database (Free)
- **API:** OMDB API (free tier: 1000 req/day)
- **Endpoint:** `http://www.omdbapi.com/?t={title}&apikey={key}`
- **Returns:** Full movie details (actors, director, year, genres, plot)

Alternative: TMDb API (better data, requires API key)

## Backend Implementation

### 1. Barcode Lookup Service
File: `dvd_catalog_api/src/barcode.rs`

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct UpcResponse {
    items: Vec<UpcItem>,
}

#[derive(Debug, Deserialize)]
struct UpcItem {
    title: String,
    brand: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OmdbResponse {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Year")]
    year: String,
    #[serde(rename = "Director")]
    director: String,
    #[serde(rename = "Actors")]
    actors: String,
    #[serde(rename = "Genre")]
    genre: String,
    #[serde(rename = "Plot")]
    plot: String,
    #[serde(rename = "Poster")]
    poster: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MovieInfo {
    pub title: String,
    pub year: i32,
    pub director: String,
    pub actors: Vec<String>,
    pub genres: Vec<String>,
    pub description: String,
    pub poster_url: Option<String>,
}

pub async fn lookup_barcode(barcode: &str) -> Result<MovieInfo, Box<dyn std::error::Error>> {
    let client = Client::new();
    
    // Step 1: UPC lookup to get product name
    let upc_url = format!("https://api.upcitemdb.com/prod/trial/lookup?upc={}", barcode);
    let upc_response: UpcResponse = client
        .get(&upc_url)
        .send()
        .await?
        .json()
        .await?;
    
    let product_title = upc_response
        .items
        .first()
        .ok_or("No product found for barcode")?
        .title
        .clone();
    
    // Step 2: Clean title (remove "DVD", "Blu-ray", etc)
    let clean_title = clean_movie_title(&product_title);
    
    // Step 3: OMDB lookup for movie details
    let omdb_key = std::env::var("OMDB_API_KEY")
        .unwrap_or_else(|_| "your_key_here".to_string());
    let omdb_url = format!(
        "http://www.omdbapi.com/?t={}&apikey={}",
        urlencoding::encode(&clean_title),
        omdb_key
    );
    
    let omdb_response: OmdbResponse = client
        .get(&omdb_url)
        .send()
        .await?
        .json()
        .await?;
    
    // Parse year
    let year = omdb_response.year
        .split('–')  // Handle ranges like "2019–2020"
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    // Parse actors (comma-separated)
    let actors: Vec<String> = omdb_response.actors
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    // Parse genres (comma-separated)
    let genres: Vec<String> = omdb_response.genre
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    Ok(MovieInfo {
        title: omdb_response.title,
        year,
        director: omdb_response.director,
        actors,
        genres,
        description: omdb_response.plot,
        poster_url: omdb_response.poster,
    })
}

fn clean_movie_title(title: &str) -> String {
    title
        .replace(" DVD", "")
        .replace(" Blu-ray", "")
        .replace(" Blu-Ray", "")
        .replace(" [Blu-ray]", "")
        .replace(" (DVD)", "")
        .replace(" - DVD", "")
        .trim()
        .to_string()
}
```

### 2. API Endpoint
File: `dvd_catalog_api/src/lib.rs`

```rust
#[derive(Deserialize)]
pub struct BarcodeLookupRequest {
    pub barcode: String,
}

pub async fn lookup_barcode_endpoint(
    Json(request): Json<BarcodeLookupRequest>,
) -> Result<Json<MovieInfo>, StatusCode> {
    match barcode::lookup_barcode(&request.barcode).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => {
            eprintln!("Barcode lookup error: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// Add route
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/barcode/lookup", post(lookup_barcode_endpoint))
        // ... other routes
        .with_state(state)
}
```

## Frontend Implementation

### 1. Camera Access Hook
File: `dvd_categorizer_web/src/hooks/use_camera.rs`

```rust
use dioxus::prelude::*;
use web_sys::{MediaStream, MediaStreamConstraints};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

pub fn use_camera() -> Signal<Option<MediaStream>> {
    let mut stream = use_signal(|| None::<MediaStream>);
    
    let start_camera = move || {
        spawn(async move {
            let window = web_sys::window().unwrap();
            let navigator = window.navigator();
            let media_devices = navigator.media_devices().unwrap();
            
            let mut constraints = MediaStreamConstraints::new();
            constraints.video(&JsValue::from(true));
            
            match JsFuture::from(media_devices.get_user_media_with_constraints(&constraints).unwrap()).await {
                Ok(stream_js) => {
                    let media_stream: MediaStream = stream_js.into();
                    stream.set(Some(media_stream));
                }
                Err(e) => {
                    log::error!("Camera access denied: {:?}", e);
                }
            }
        });
    };
    
    stream
}
```

### 2. Barcode Scanner Component
File: `dvd_categorizer_web/src/components/barcode_scanner.rs`

```rust
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ZXing)]
    type BrowserMultiFormatReader;
    
    #[wasm_bindgen(constructor, js_namespace = ZXing)]
    fn new() -> BrowserMultiFormatReader;
    
    #[wasm_bindgen(method, js_namespace = ZXing)]
    fn decodeFromVideoDevice(
        this: &BrowserMultiFormatReader,
        device_id: Option<String>,
        video_element: &web_sys::HtmlVideoElement,
        callback: &Closure<dyn FnMut(JsValue, JsValue)>,
    );
}

#[component]
pub fn BarcodeScanner(on_scan: EventHandler<String>) -> Element {
    let mut scanning = use_signal(|| false);
    let mut last_scan = use_signal(|| String::new());
    
    let start_scan = move |_| {
        scanning.set(true);
        
        spawn(async move {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let video = document
                .get_element_by_id("barcode-video")
                .unwrap()
                .dyn_into::<web_sys::HtmlVideoElement>()
                .unwrap();
            
            let reader = BrowserMultiFormatReader::new();
            
            let callback = Closure::wrap(Box::new(move |result: JsValue, _err: JsValue| {
                if let Some(text) = result.as_string() {
                    if text != last_scan() {
                        last_scan.set(text.clone());
                        on_scan.call(text);
                    }
                }
            }) as Box<dyn FnMut(JsValue, JsValue)>);
            
            reader.decodeFromVideoDevice(None, &video, &callback);
            callback.forget();
        });
    };
    
    rsx! {
        div { class: "barcode-scanner",
            if scanning() {
                div { class: "scanner-view",
                    video {
                        id: "barcode-video",
                        autoplay: true,
                        playsinline: true,
                    }
                    div { class: "scanner-overlay",
                        div { class: "scan-line" }
                    }
                    button {
                        class: "stop-scan-btn",
                        onclick: move |_| scanning.set(false),
                        "Stop Scanning"
                    }
                }
            } else {
                button {
                    class: "start-scan-btn",
                    onclick: start_scan,
                    "📷 Scan Barcode"
                }
            }
        }
    }
}
```

### 3. Movie Entry Form with Scanner
File: `dvd_categorizer_web/src/components/add_movie.rs`

```rust
#[component]
pub fn AddMovieForm() -> Element {
    let mut movie_info = use_signal(|| None::<MovieInfo>);
    let mut loading = use_signal(|| false);
    
    let handle_barcode_scan = move |barcode: String| {
        loading.set(true);
        
        spawn(async move {
            match lookup_movie_by_barcode(&barcode).await {
                Ok(info) => {
                    movie_info.set(Some(info));
                    loading.set(false);
                }
                Err(e) => {
                    log::error!("Lookup failed: {}", e);
                    loading.set(false);
                }
            }
        });
    };
    
    rsx! {
        div { class: "add-movie-form",
            h2 { "Add New Movie" }
            
            BarcodeScanner { on_scan: handle_barcode_scan }
            
            if loading() {
                div { class: "loading", "Looking up movie..." }
            }
            
            if let Some(info) = movie_info() {
                div { class: "movie-preview",
                    if let Some(poster) = &info.poster_url {
                        img { src: "{poster}", alt: "Movie poster" }
                    }
                    
                    h3 { "{info.title}" }
                    p { "Year: {info.year}" }
                    p { "Director: {info.director}" }
                    p { "Actors: {info.actors.join(\", \")}" }
                    p { "Genres: {info.genres.join(\", \")}" }
                    p { class: "description", "{info.description}" }
                    
                    div { class: "form-actions",
                        button {
                            class: "btn-primary",
                            onclick: move |_| save_movie(info.clone()),
                            "Add to Library"
                        }
                        button {
                            class: "btn-secondary",
                            onclick: move |_| movie_info.set(None),
                            "Scan Another"
                        }
                    }
                }
            }
        }
    }
}

async fn lookup_movie_by_barcode(barcode: &str) -> Result<MovieInfo, Box<dyn std::error::Error>> {
    let response = gloo_net::http::Request::post("/api/barcode/lookup")
        .json(&BarcodeLookupRequest {
            barcode: barcode.to_string(),
        })?
        .send()
        .await?;
    
    response.json().await.map_err(Into::into)
}

async fn save_movie(info: MovieInfo) {
    // Convert to FullMovie and save via existing API
    // ...
}
```

## Mobile Considerations

### Progressive Web App (PWA)
Add to `index.html`:
```html
<link rel="manifest" href="/manifest.json">
<meta name="theme-color" content="#0891b2">
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1">
```

File: `public/manifest.json`
```json
{
  "name": "DVD Catalog",
  "short_name": "DVD Cat",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#1f2937",
  "theme_color": "#0891b2",
  "icons": [
    {
      "src": "/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ],
  "permissions": ["camera"]
}
```

## Benefits

- ✓ Scan barcode → instant movie info
- ✓ No manual typing
- ✓ Accurate data from OMDB
- ✓ Works on phone (PWA)
- ✓ Poster images included
- ✓ One-click add to library

## Estimated Time
4-5 hours (API integration + camera access + barcode scanning + testing)

## Testing

1. Scan known DVD barcode, verify correct movie
2. Scan non-movie barcode, verify error handling
3. Test on mobile device (camera access)
4. Test with poor lighting (barcode quality)
5. Test offline behavior (API failures)

## Alternative: Manual Barcode Entry

If camera access problematic, add manual input:
```rust
input {
    r#type: "text",
    placeholder: "Enter barcode manually",
    oninput: move |evt| handle_barcode_scan(evt.value()),
}
```

## API Rate Limits

**UPCitemdb Free Tier:**
- 100 requests/day
- Upgrade: $10/month = 10,000 req/day

**OMDB Free Tier:**
- 1,000 requests/day
- Upgrade: $1/month = 100,000 req/day

**Solution for heavy use:**
- Cache lookups in DB (barcode → movie_id)
- Batch import mode (scan all, lookup once)

## Future Enhancements

- Bulk scan mode (scan multiple DVDs, batch process)
- Offline mode (cache common barcodes)
- Manual edit if lookup wrong
- Alternative APIs (TMDb, Google Books for rare titles)
- ISBN support (for box sets with ISBN instead of UPC)
