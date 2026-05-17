# Module 10 - Asynchronous Programming

# Tutorial 3: WebChat using yew

> DISCLAIMER: This app is a modern implementation of this original tutorial at [https://blog.devgenius.io/lets-build-a-websockets-project-with-rust-and-yew-0-19-60720367399f](https://blog.devgenius.io/lets-build-a-websockets-project-with-rust-and-yew-0-19-60720367399f) which is now outdated and incompatible with modern Rust toolchains. All credits for the original project and design go to the author, but the code has been completely rewritten from scratch to work with the latest versions of Yew, Trunk, and modern Rust async patterns.

## How to run

- Ensure you have Rust and Trunk installed. If not, install Rust from [https://rustup.rs/](https://rustup.rs/) and Trunk using `cargo install trunk`.
- Clone the repository and navigate to the project directory.
- Make sure you have an existing WebSocket server running on port 8080. [The old tutorial suggests using this server. Follow the instructions in the repo to set it up.](https://github.com/jtordgeman/SimpleWebsocketServer)
- In the project directory, run `trunk serve --open` to start the YewChat application.
- The application should automatically open in your default web browser at `http://localhost:8000`. You can start sending messages through the chat interface, and they will be relayed to the WebSocket server on port 8080.
- Open more browser tabs to `http://localhost:8000` to simulate multiple clients and see the real-time chat functionality in action.

## Reflection 3.1 Original code (refactored for modern Rust usage)

![Screenshot of the YewChat application showing a simple chat interface with messages and an input box, and also the terminal running the web server on port 8080](assets/3.1.png)

While following [the original tutorial](https://blog.devgenius.io/lets-build-a-websockets-project-with-rust-and-yew-0-19-60720367399f) for building the YewChat application, I encountered several insurmountable compatibility issues due to the rapidly evolving Rust WebAssembly ecosystem. The original guide relied on outdated libraries and tools (such as Yew 0.19, reqwasm, yew-agent, and an older Webpack configuration) which resulted in critical WASM parsing errors and incompatible dependencies when compiled with modern Rust toolchains.

To resolve this, I decided to build the application from scratch using a modern approach, ensuring it runs perfectly on the latest standards. Here are the key differences and improvements made during the refactoring process:

- Switched from Webpack to Trunk:

    The original tutorial used Webpack and `npm` to bundle the application, which failed to parse modern WASM Reference Types. I migrated the build system to Trunk, a zero-config WASM bundler for Rust. This completely eliminated the parser errors, removed the need for Node.js configurations in the frontend, and allowed me to easily bind the local server to port `8000` (to avoid port collision with [the NodeJS WebSocket server](https://github.com/jtordgeman/SimpleWebsocketServer) on `8080`).

- Upgraded to Yew 0.21 and Modern Hooks:

    Instead of using older component lifecycles and manually wrapping states in `Rc<RefCell<...>>`, the refactored code leverages modern Yew hooks like `use_state` and `use_context`. For the chat messages array, I implemented `use_mut_ref` to prevent a "stale closure" bug inside the asynchronous loop, ensuring the UI always appends and renders the most up-to-date messages.

- Replaced Event Bus Architecture with `gloo-net`:

    The old implementation relied heavily on `yew-agent` and `mpsc::channel` to create a complex Event Bus acting as a middleman between the WebSocket service and the UI components. In my modern implementation, I discarded this complexity. Instead, I used `gloo-net` to establish a direct WebSocket connection inside a use_effect_with hook within the Chat component itself. Messages are handled asynchronously using `wasm_bindgen_futures::spawn_local`, making the code significantly cleaner and easier to maintain.

- Handling Race Conditions and Payload Formatting:

    Modern Rust executes asynchronous tasks extremely fast. I encountered a race condition where the Rust client attempted to send the Register payload while the socket was still in the `CONNECTING` state, causing the NodeJS server to instantly drop the connection (Code 1006). I solved this by introducing `gloo-timers` to implement a safe 500ms delay, allowing the handshake to complete. Furthermore, I utilized `#[serde(skip_serializing_if = "Option::is_none")]` to dynamically omit the data_array field from the JSON payload when it's not needed, successfully preventing the NodeJS server from crashing due to null value evaluations.

Overall, this refactoring experience provided deep insights into how much the Rust WebAssembly ecosystem has matured, favoring simpler hook-based architectures and dedicated WASM tooling over complex JavaScript bundler integrations.

## Reflection 3.2 Add some creativities to the webclient

![Screenshot of the YewChat application showing the new image upload feature, emoji picker, and light/dark theme](assets/3.2.png)

To elevate the user experience and explore the full potential of Yew and WebAssembly, I implemented three major creative features on top of the refactored modern YewChat application. These features focus on interactivity, accessibility, and client-side performance optimization.

- Interactive Emoji Selector

    I added a Unicode-based emoji selector to make the chat more expressive. This feature utilizes a boolean state (`use_state`) to toggle the visibility of an absolute-positioned floating popup menu over the chat input. When an emoji button is clicked, a callback appends the specific Unicode character to the existing `chat_input` state. This demonstrates seamless real-time state manipulation without unnecessary re-renders.

- Dynamic Dark/Light Theme with System Detection

    To improve accessibility and user comfort, I implemented a dynamic theming system that automatically adapts to the user's operating system preferences, along with a manual toggle. I utilized the `web-sys` crate to access the browser's Window API and evaluate the `(prefers-color-scheme: dark)` media query during the initial component render. The result dictates the initial boolean value of the `is_dark state`. The UI rendering logic then dynamically injects specific Tailwind CSS classes (e.g., swapping `bg-white` with `bg-gray-900`) based on this state. This provides a premium, flicker-free theming experience entirely driven by Rust.

- Image Attachment with Client-Side Compression

    To simulate a production-ready chat application, I added image upload capabilities. The stock implementation already supported online GIFs, but I extended it to support local image uploads as well. However, to prevent overloading the simple NodeJS broadcast server with massive file payloads, I implemented a strict client-side compression pipeline. This is achieved by heavily leveraging `web-sys`, `js-sys`, and `wasm-bindgen` to interact directly with the HTML5 DOM API from Rust.
    - A hidden `<input type="file">` captures the image.
    - A `FileReader` loads the file into an `HtmlImageElement`.
    - The image is drawn onto an in-memory `HtmlCanvasElement`. The Rust logic calculates the aspect ratio to downscale the image so that its maximum dimension does not exceed 800 pixels.
    - The canvas exports the resized image as a JPEG Data URL (Base64) with a quality factor of 0.7, drastically reducing the file size (typically well under 500 KB).
    - The resulting Base64 string is safely sent via the WebSocket Message payload.

## Bonus: Refactoring the Broadcast Chat Server (Tutorial 2) to Support YewChat

**How it is done:**

To make the Rust-based WebSocket server from Tutorial 2 fully compatible with the modern YewChat client, I completely refactored its message-handling logic to act as a robust JSON relay.

- I added `serde` and `serde_json` to the server's Cargo.toml.

- I defined identical Rust structs (`WebSocketMessage`, `MsgTypes`, and `MessageData`) on the server side to match the exact JSON schema expected by the YewChat client.

- I implemented an `Arc<Mutex<HashMap<SocketAddr, String>>>` to keep track of active connections and bind each socket to a registered username.

- Instead of broadcasting plain text, the server now parses incoming JSON. When it receives a `Message` event, it extracts the content (whether it's text, emojis, or Base64 image data), encapsulates it inside a `MessageData` object alongside the sender's name, wraps it back into a `WebSocketMessage`, and safely broadcasts it to all connected clients.

**Why it is a successful change:**
This change is highly successful because it bridges the gap between the backend and our newly modernized frontend seamlessly. Unlike the provided SimpleWebsocketServer (NodeJS) which crashed when receiving unexpected null arrays, this Rust server strictly validates incoming payloads using `serde`. It successfully handles complex data types—including the Base64 image strings from our creative client-side compression feature—without any performance degradation or data corruption. The entire application is now a unified, Full-Stack Asynchronous Rust architecture.

**Opinion: JavaScript Version vs. Rust Version**
I strongly prefer the Rust version. The primary reason is its unmatched performance and strict type safety. Dealing with the JavaScript server was tedious. JavaScript is highly prone to sudden runtime errors (such as the classic `TypeError: Cannot read properties of null`) that can instantly crash the entire server without warning. This is made worse with having to deal with `npm` dependencies and their often unpredictable updates.

In contrast, Rust forces you to handle every possible state (like `Option::None` or `Result::Err`) at compile time. By the time the Rust server compiles successfully, I have the absolute confidence that it will run safely, predictably, and efficiently without randomly breaking mid-execution. Rust eliminates the messiness of JavaScript, making the development process much more robust and reliable.
