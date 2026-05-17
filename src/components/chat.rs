use serde::{Deserialize, Serialize};
use yew::prelude::*;
use wasm_bindgen::JsCast;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::HtmlInputElement;
use crate::User;
use gloo_timers::future::TimeoutFuture;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data_array: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Clone, Debug, PartialEq)]
struct UserProfile {
    name: String,
    avatar: String,
}

#[function_component(Chat)]
pub fn chat() -> Html {
    let user_ctx = use_context::<UseStateHandle<User>>().expect("No context found");
    let username = user_ctx.username.clone();

    let users = use_state(Vec::<UserProfile>::new);
    let chat_input = use_state(String::new);

    let messages = use_mut_ref(Vec::<MessageData>::new);
    let render_trigger = use_state(|| 0);

    let show_emoji = use_state(|| false);
    let emoji_list = vec!["😀", "😂", "🥰", "😎", "😭", "😡", "👍", "🙏", "👀", "✨", "🔥", "❤️", "🎉", "🤔", "🥳", "🤯", "😳", "👌", "🙌"];

    type WsSender = Rc<RefCell<SplitSink<WebSocket, Message>>>;
    let ws_sender = use_state(|| None::<WsSender>);

    {
        let users = users.clone();
        let messages = messages.clone();
        let render_trigger = render_trigger.clone();
        let ws_sender = ws_sender.clone();
        let username = username.clone();

        use_effect_with((), move |_| {
            let ws = WebSocket::open("ws://127.0.0.1:8080").expect("Gagal membuka WebSocket");
            let (mut write, mut read) = ws.split();

            let register_msg = WebSocketMessage {
                message_type: MsgTypes::Register,
                data: Some(username),
                data_array: None,
            };

            wasm_bindgen_futures::spawn_local(async move {
                TimeoutFuture::new(500).await;
                let json_str = serde_json::to_string(&register_msg).unwrap();
                let _ = write.send(Message::Text(json_str)).await;
                
                ws_sender.set(Some(Rc::new(RefCell::new(write))));
            });

            wasm_bindgen_futures::spawn_local(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                            match ws_msg.message_type {
                                MsgTypes::Users => {
                                    if let Some(user_list) = ws_msg.data_array {
                                        let profiles = user_list.into_iter()
                                            .filter(|name| !name.trim().is_empty()) 
                                            .map(|name| UserProfile {
                                                avatar: format!("https://api.dicebear.com/7.x/adventurer/svg?seed={}", name),
                                                name,
                                            }).collect();
                                        users.set(profiles);
                                    }
                                }
                                MsgTypes::Message => {
                                    if let Some(msg_data_str) = ws_msg.data {
                                        if let Ok(msg_data) = serde_json::from_str::<MessageData>(&msg_data_str) {
                                            messages.borrow_mut().push(msg_data);
                                            render_trigger.set(*render_trigger + 1);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
            || ()
        });
    }

    let oninput = {
        let chat_input = chat_input.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            chat_input.set(input.value());
        })
    };

    let onsubmit = {
        let chat_input = chat_input.clone();
        let ws_sender = ws_sender.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let msg_text = (*chat_input).clone();
            if msg_text.is_empty() { return; }

            if let Some(sender_rc) = &*ws_sender {
                let msg_obj = WebSocketMessage {
                    message_type: MsgTypes::Message,
                    data: Some(msg_text),
                    data_array: None,
                };
                let json_str = serde_json::to_string(&msg_obj).unwrap();
                
                let sender_rc = Rc::clone(sender_rc);
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = sender_rc.borrow_mut().send(Message::Text(json_str)).await;
                });
            }
            chat_input.set(String::new());
        })
    };

    let on_toggle_emoji = {
        let show_emoji = show_emoji.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            show_emoji.set(!*show_emoji);
        })
    };

    let on_emoji_click = {
        let chat_input = chat_input.clone();
        let show_emoji = show_emoji.clone();
        move |emoji: String| {
            let chat_input = chat_input.clone();
            let show_emoji = show_emoji.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                chat_input.set(format!("{}{}", *chat_input, emoji));
                show_emoji.set(false); 
            })
        }
    };

    let is_dark = use_state(|| {
        web_sys::window()
            .and_then(|win| win.match_media("(prefers-color-scheme: dark)").ok().flatten())
            .map(|media| media.matches())
            .unwrap_or(false)
    });

    let on_toggle_theme = {
        let is_dark = is_dark.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            is_dark.set(!*is_dark);
        })
    };

    let file_input_ref = use_node_ref();

    let on_attach_click = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_file_change = {
        let ws_sender = ws_sender.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    if let Some(sender_rc) = &*ws_sender {
                        let sender_rc = Rc::clone(sender_rc);
                        
                        let reader = web_sys::FileReader::new().unwrap();
                        let reader_clone = reader.clone();
                        
                        let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                            let result = reader_clone.result().unwrap().as_string().unwrap();
                            
                            let img = web_sys::HtmlImageElement::new().unwrap();
                            let img_clone = img.clone();
                            let sender_rc_inner = Rc::clone(&sender_rc);
                            
                            let img_onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                                let window = web_sys::window().unwrap();
                                let document = window.document().unwrap();
                                let canvas = document.create_element("canvas").unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
                                
                                let max_dim = 800.0;
                                let mut width = img_clone.width() as f64;
                                let mut height = img_clone.height() as f64;
                                
                                if width > max_dim || height > max_dim {
                                    if width > height {
                                        height *= max_dim / width;
                                        width = max_dim;
                                    } else {
                                        width *= max_dim / height;
                                        height = max_dim;
                                    }
                                }
                                
                                canvas.set_width(width as u32);
                                canvas.set_height(height as u32);
                                
                                let ctx = canvas.get_context("2d").unwrap().unwrap().dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap();
                                ctx.draw_image_with_html_image_element_and_dw_and_dh(&img_clone, 0.0, 0.0, width, height).unwrap();
                                
                                let quality = wasm_bindgen::JsValue::from_f64(0.7);
                                let data_url = canvas.to_data_url_with_type_and_encoder_options("image/jpeg", &quality).unwrap();
                                
                                let msg_obj = WebSocketMessage {
                                    message_type: MsgTypes::Message,
                                    data: Some(data_url),
                                    data_array: None,
                                };
                                let json_str = serde_json::to_string(&msg_obj).unwrap();
                                
                                let sender = Rc::clone(&sender_rc_inner);
                                wasm_bindgen_futures::spawn_local(async move {
                                    let _ = sender.borrow_mut().send(Message::Text(json_str)).await;
                                });
                            }) as Box<dyn FnMut(_)>);
                            
                            img.set_onload(Some(img_onload.as_ref().unchecked_ref()));
                            img_onload.forget();
                            
                            img.set_src(&result);
                        }) as Box<dyn FnMut(_)>);
                        
                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                        onload.forget();
                        
                        reader.read_as_data_url(&file).unwrap();
                    }
                }
            }
            input.set_value("");
        })
    };
    
    let bg_main = if *is_dark { "bg-gray-900" } else { "bg-white" };
    let bg_sidebar = if *is_dark { "bg-gray-800 border-gray-700 text-white" } else { "bg-gray-100 border-gray-300 text-gray-800" };
    let bg_sidebar_item = if *is_dark { "bg-gray-700 border-gray-600 text-white" } else { "bg-white border-gray-200 text-gray-800" };
    let bg_header = if *is_dark { "bg-gray-800 border-gray-700 text-white" } else { "bg-gray-50 border-gray-300 text-gray-700" };
    let bg_chat_area = if *is_dark { "bg-gray-900" } else { "bg-gray-50" };
    let bg_msg_other = if *is_dark { "bg-gray-700 text-gray-100 border-gray-600" } else { "bg-white text-gray-800 border-gray-200" };
    let bg_input_area = if *is_dark { "bg-gray-800 border-gray-700" } else { "bg-white border-gray-300" };
    let bg_input_box = if *is_dark { "bg-gray-700 text-white placeholder-gray-400" } else { "bg-gray-100 text-gray-800" };
    let bg_emoji_popup = if *is_dark { "bg-gray-800 border-gray-700 text-white" } else { "bg-white border-gray-200" };
    
    html! {
        <div class={format!("flex w-screen h-screen {}", bg_main)}>
            <div class={format!("flex-none w-64 h-screen border-r flex flex-col {}", bg_sidebar)}>
                <div class="text-xl p-4 font-bold border-b border-inherit">{"Users"}</div>
                <div class="overflow-y-auto grow p-2 space-y-2">
                    { for users.iter().map(|u| html! {
                        <div class={format!("flex items-center rounded p-2 shadow-sm border {}", bg_sidebar_item)}>
                            <img class="w-10 h-10 rounded-full bg-gray-200" src={u.avatar.clone()} />
                            <div class="ml-3 font-medium">{u.name.clone()}</div>
                        </div>
                    }) }
                </div>
            </div>

            <div class="grow h-screen flex flex-col">
                <div class={format!("w-full h-14 border-b-2 flex items-center justify-between px-4 font-bold {}", bg_header)}>
                    <span>{"💬 YewChat"}</span>
                    <button 
                        onclick={on_toggle_theme} 
                        class="px-3 py-1 text-sm rounded-full bg-gray-500/20 hover:bg-gray-500/40 transition-colors"
                    >
                        if *is_dark { {"☀️ Light"} } else { {"🌙 Dark"} }
                    </button>
                </div>
                
                <div class={format!("w-full grow overflow-y-auto p-4 flex flex-col space-y-4 {}", bg_chat_area)}>
                    { for messages.borrow().iter().map(|m| {
                        let is_me = m.from == username;
                        let avatar = users.iter()
                            .find(|u| u.name == m.from)
                            .map(|u| u.avatar.clone())
                            .unwrap_or_else(|| format!("https://api.dicebear.com/7.x/adventurer/svg?seed={}", m.from));

                        html! {
                            <div class={format!("flex items-end w-3/4 {}", if is_me { "self-end flex-row-reverse" } else { "" })}>
                                <img class={format!("w-8 h-8 rounded-full bg-gray-200 shadow-sm {}", if is_me { "ml-3" } else { "mr-3" })} src={avatar} />
                                <div class={format!("p-3 rounded-lg shadow-sm border {}", if is_me { "bg-blue-500 text-white border-blue-600" } else { bg_msg_other })}>
                                    <div class={format!("text-xs font-bold mb-1 {}", if is_me { "text-blue-100" } else { "opacity-70" })}>
                                        {m.from.clone()}
                                    </div>
                                    <div class="text-sm break-words">
                                        if m.message.starts_with("data:image/") || m.message.ends_with(".gif") {
                                            <img class="mt-2 rounded max-w-sm max-h-64 object-contain shadow-sm" src={m.message.clone()} />
                                        } else {
                                            {m.message.clone()}
                                        }
                                    </div>
                                </div>
                            </div>
                        }
                    }) }
                </div>

                <div class="relative w-full">
                    if *show_emoji {
                        <div class={format!("absolute bottom-20 left-4 border shadow-lg rounded-lg p-3 w-64 z-50 {}", bg_emoji_popup)}>
                            <div class="grid grid-cols-5 gap-2">
                                { for emoji_list.into_iter().map(|emoji| {
                                    let emoji_clone = emoji.to_string();
                                    html! {
                                        <button 
                                            type="button" 
                                            onclick={on_emoji_click(emoji_clone)}
                                            class="text-2xl hover:bg-gray-500/30 rounded p-1 transition-colors cursor-pointer"
                                        >
                                            {emoji}
                                        </button>
                                    }
                                }) }
                            </div>
                        </div>
                    }

                    <form onsubmit={onsubmit} class={format!("w-full h-16 border-t-2 flex px-4 items-center {}", bg_input_area)}>
                        <input 
                            type="file" 
                            accept="image/*" 
                            ref={file_input_ref} 
                            onchange={on_file_change} 
                            class="hidden" 
                        />
                        <button 
                            type="button" 
                            onclick={on_attach_click}
                            class="p-2 text-2xl opacity-70 hover:opacity-100 transition-opacity cursor-pointer"
                            title="Attach Image"
                        >
                            {"🖼️"}
                        </button>
                        <button 
                            type="button" 
                            onclick={on_toggle_emoji}
                            class="p-2 text-2xl opacity-70 hover:opacity-100 transition-opacity cursor-pointer mr-1"
                        >
                            {"😀"}
                        </button>
                        <input 
                            {oninput}
                            value={(*chat_input).clone()}
                            type="text" 
                            placeholder="Type a message..." 
                            class={format!("block grow py-2 pl-4 mx-3 rounded-full outline-none focus:ring-2 focus:ring-blue-500 transition-colors {}", bg_input_box)} 
                            required=true 
                        />
                        <button type="submit" class="p-2 bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center text-white hover:bg-blue-500 shadow-sm transition-colors">
                            <svg fill="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="w-5 h-5">
                                <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                            </svg>
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}