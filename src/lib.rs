use worker::*;

mod utils;
mod relay;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    let namespace = env.durable_object("RELAY")?;
    let id = namespace.id_from_name("relay")?;
    let relay = id.get_stub()?;
    relay.fetch_with_request(req).await
}
