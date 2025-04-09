use pipewire::stream::{Stream, StreamListener};
use tokio::main;
use xdg_portal::common::SourceType;
use xdg_portal::portal::Portal;
use xdg_portal::screencast::ScreencastReq;

#[main(flavor = "current_thread")]
async fn main() {
    let portal = Portal::new().await.unwrap();
    let mut screencast_portal = portal.screencast().await.unwrap();
    let screencast_req = ScreencastReq::new().source_type(SourceType::Window | SourceType::Monitor);
    let res = match screencast_portal.screencast(screencast_req).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return;
        }
    };
    // YES, THE RESPONSE WORKS, BUT I HAVE NO FUCKING CLUE WHAT TO USE IT FOR
    println!("Screencast response: {:?}", res);
    let main_loop = pipewire::main_loop::MainLoop::new(None).unwrap();
    let context = pipewire::context::Context::new(&main_loop).unwrap();
}
