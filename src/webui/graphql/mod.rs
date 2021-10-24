use super::{util::SessionUser, WebUI};
use juniper::{EmptySubscription, RootNode};
use juniper_rocket::GraphQLRequest;
use rocket::{
    get,
    outcome::try_outcome,
    post,
    request::{FromRequest, Outcome, Request},
    response::content::Html,
    routes, uri, Build, Rocket, State,
};
use serenity::model::user::CurrentUser;

mod guilds;
mod query;
pub use query::{types, Mutation, Query};

pub struct Context {
    pub webui: WebUI,
    pub user: Option<CurrentUser>,
}

impl juniper::Context for Context {}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Context {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let webui = try_outcome!(request.guard::<&State<WebUI>>().await);
        let user = request.guard::<Option<SessionUser<'_>>>().await.unwrap(); // Option always results in Success
        Outcome::Success(Context {
            webui: webui.inner().clone(),
            user: user.map(|u| u.into_current_user().clone()),
        })
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.manage(Schema::new(Query, Mutation, EmptySubscription::new()))
        .mount(
            "/",
            routes![graphiql, playground, graphql_get, graphql_post],
        )
}

#[get("/graphiql")]
fn graphiql() -> Html<String> {
    juniper_rocket::graphiql_source(&uri!(graphql_post).to_string(), None)
}

#[get("/playground")]
fn playground() -> Html<String> {
    juniper_rocket::playground_source(&uri!(graphql_post).to_string(), None)
}

#[get("/api/graphql?<request>")]
async fn graphql_get(
    request: GraphQLRequest,
    schema: &State<Schema>,
    context: Context,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&*schema, &context).await
}

#[post("/api/graphql", data = "<request>")]
async fn graphql_post(
    request: GraphQLRequest,
    schema: &State<Schema>,
    context: Context,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&*schema, &context).await
}
