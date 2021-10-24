use super::{util::SessionUser, WebUI};
use juniper::{EmptySubscription, RootNode};
use juniper_rocket::GraphQLRequest;
use rocket::{get, post, response::content::Html, routes, Build, Rocket, State};
use serenity::model::user::CurrentUser;

mod guilds;
mod query;
use query::{Mutation, Query};

pub struct Context {
    webui: WebUI,
    user: Option<CurrentUser>,
}

impl juniper::Context for Context {}

type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.manage(Schema::new(Query, Mutation, EmptySubscription::new()))
        .mount("/", routes![graphiql])
        .mount("/api", routes![graphql_get, graphql_post])
}

#[get("/graphiql")]
fn graphiql() -> Html<String> {
    // TODO: use uri! macro
    juniper_rocket::graphiql_source("/api/graphql", None)
}

async fn graphql(
    request: GraphQLRequest,
    schema: &State<Schema>,
    webui: &State<WebUI>,
    user: Option<SessionUser<'_>>,
) -> juniper_rocket::GraphQLResponse {
    let context = Context {
        webui: webui.inner().clone(),
        user: user.map(|u| u.into_current_user().clone()),
    };
    request.execute(&*schema, &context).await
}

#[get("/graphql?<request>")]
async fn graphql_get(
    request: GraphQLRequest,
    schema: &State<Schema>,
    webui: &State<WebUI>,
    user: Option<SessionUser<'_>>,
) -> juniper_rocket::GraphQLResponse {
    graphql(request, schema, webui, user).await
}

#[post("/graphql", data = "<request>")]
async fn graphql_post(
    request: GraphQLRequest,
    schema: &State<Schema>,
    webui: &State<WebUI>,
    user: Option<SessionUser<'_>>,
) -> juniper_rocket::GraphQLResponse {
    graphql(request, schema, webui, user).await
}
