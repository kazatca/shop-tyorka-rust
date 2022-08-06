use actix_web::{self, guard, middleware, web, App, HttpServer};
use actix_web_grants::permissions::AuthDetails;
use actix_web_grants::GrantsMiddleware;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use grant::{Role, RoleData};
use std::io;

mod api;
mod config;
mod entity;
mod grant;
mod graphql_schema;
mod services;

use crate::api::{ApiClient, GraphQLClient};
use crate::grant::extract;
use graphql_schema::{build_schema, GQLSchema};
use services::{OrderService, OrderServiceMethods, TgBot, TgBotExt, Recaptcha, RecaptchaMethods};

async fn index(
    schema: web::Data<GQLSchema>,
    request: GraphQLRequest,
    role: AuthDetails<Role>,
) -> GraphQLResponse {
    schema
        .execute(request.into_inner().data(RoleData(role)))
        .await
        .into()
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let cfg = config::load();
    let port = cfg.port.clone();

    HttpServer::new(move || {
        let auth = GrantsMiddleware::with_extractor(extract);
        let api_client = ApiClient::new(&cfg.api_client).unwrap();
        let order_service = OrderService::new(api_client.clone(), TgBot::new(&cfg.tg_client));
        let recaptcha = Recaptcha::new(&cfg.recaptcha);

        let schema = build_schema().data(api_client).data(order_service).data(recaptcha).finish();

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(auth)
            .app_data(web::Data::new(cfg.clone()))
            .app_data(web::Data::new(schema.clone()))
            .service(web::resource("/graphql").guard(guard::Post()).to(index))
    })
    .bind(format!("0.0.0.0:{port}", port = port))?
    .run()
    .await
}
