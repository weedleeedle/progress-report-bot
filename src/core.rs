//! Small module to maintain global command data
//! that is passed through to every command.
//! Global command data includes things like the Discord client,
//! the scheduler, and the database donnection pool.

use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;
use anyhow::Result;

/// This struct holds global data that is passed into every command.
/// Essentially manages/passes in global state/data.
pub struct GlobalCommandData {
    /// Reference to the database pool.
    /// Used for everything you'd do with a database.
    /// Note that PgPool *already* implemnts Arc and can is intended 
    /// to be cloned across threads and all over the place
    db_pool: PgPool,
    /*
    /// Reference to CacheAndHttp, which allows us to interact with REST Api.
    /// Cache can cache results so that less API calls are required.
    // I considered using some form of [serenity::Client] here
    // but I am not sure if that's better than this.
    // This may change later lol.
    // This is why I'm leaving it as "client" though.
    //
    // This MUST be set when this struct is created!!!!
    client: Option<Arc<serenity::CacheAndHttp>>
    */
}

impl GlobalCommandData
{
    /*
    /// Stores a reference to the client CacheAndHttp. It doesn't actually
    /// keep a reference to the client. This may change in the future 
    /// so the nomenclature of this method is being kept.
    pub fn set_client(&mut self, client: &serenity::Client){
        self.client = Some(client.cache_and_http.clone());
    }

    /// Gets a reference to the stored CacheAndHttp.
    pub fn get_client(&self) -> &serenity::CacheAndHttp
    {
        self.client.as_ref().expect("GlobalCommandData client was not set!")
    }
    */

    /// Gets a reference to the database connection pool.
    pub fn get_pool(&self) -> &PgPool
    {
        &self.db_pool
    }
}

/// Used to initialize GlobalCommandData at the beginning of the program.
/// 
/// # Required functions
/// Some options/settinsg do NOT have defaults and MUST be set. 
/// Failure to set these will result in a MissingRequiredField
/// error from the build() command.
/// - database_url
///
///
/// Expected use case is something like the following:
/// ```no_run
/// # use progress_report_bot::core::GlobalCommandDataBuilder;
/// # #[tokio::main]
/// # async fn main() -> Result<(), anyhow::Error> {
/// let builder = GlobalCommandDataBuilder::new();
/// let global_command_data = builder
///     .max_connections(5)
///     .database_url("postgres://user:password@localhost/test".to_string())
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct GlobalCommandDataBuilder
{
    max_connections: u32,
    database_url: Option<String>,
}

/// Error struct. Returned from [build()] when
/// required data fields are not set before calling it.
#[derive(Debug,Error)]
#[error("Required field {0} was not set and has no default")]
pub struct MissingRequiredField(&'static str);

impl GlobalCommandDataBuilder 
{
    /// Creates a new GlobalCommandDataBuilder.
    /// 
    /// # Default Settings
    /// For settings that do have default options, those are set here.
    /// - max_connections = 1
    pub fn new() -> Self 
    {
        Self
        {
            max_connections: 1,
            database_url: None,
        }
    }

    /// Sets the maximum number of connections for the database pool.
    /// The default is 1 if it is not set here.
    pub fn max_connections(mut self, max_connections: u32) -> Self
    {
        self.max_connections = max_connections;
        self
    }

    /// Sets the database URL for the pool to connect to.
    /// This method MUST be called on the global command data builder.
    // This should probably return a different type teehee
    pub fn database_url(mut self, database_url: String) -> Self
    {
        self.database_url = Some(database_url);
        self
    }

    pub async fn build(&self) -> Result<GlobalCommandData>
    {
        if self.database_url.is_none()
        {
            Err(MissingRequiredField("database_url").into())
        }

        else {
            Ok(GlobalCommandData {
                db_pool: PgPoolOptions::new()
                    .max_connections(self.max_connections)
                    .connect(&self.database_url.as_ref().unwrap())
                    .await?,
                //client: None,
            })
        }
    }
}

/// Defines application configuration variables that are loaded from environment variables.
/// Instantiate this struct with [Variables::load_variables()]
pub struct Variables {
    token: String,
    max_connections: u32,
    database_url: String,
}

/// Loading variables can fail for two reasons:
/// - A required environment variable wasn't found.
/// - An environment variable was found but was in the wrong format (i.e MAX_CONNECTIONS not being
/// a u32)
/// This error is thrown by [Variables::load_variables()] in either case.
#[derive(Debug,Error)]
pub enum LoadVariablesError
{
    #[error("Missing required environment variable {0}")]
    MissingRequiredEnvironmentVariable(&'static str),
    #[error("Environment variable {0} was in an invalid format: {1}")]
    EnvironmentVariableInInvalidFormat(&'static str, &'static str),
}

impl Variables
{
    /// Loads environment variables and instantiates the [Variables] struct with them.
    ///
    /// # Errors
    ///
    /// [LoadVariablesError::MissingRequiredEnvironmentVariable] - Thrown when
    /// a required environment variable was missing. Not all used environment variables are
    /// required (i.e MAX_CONNECTIONS has a default value of 5)
    /// [LoadVariablesError::EnvironmentVariableInInvalidFormat] - Thrown when
    /// a defined environment variable was in an invalid format (i.e MAX_CONNECTIONS not being a
    /// u32)
    ///
    /// # Examples
    ///
    /// ```
    /// # use progress_report_bot::core::Variables;
    /// # use progress_report_bot::core::LoadVariablesError;
    /// # dotenvy::dotenv();
    /// let variables = Variables::load_variables()?;
    /// # Ok::<(), LoadVariablesError>(())
    /// ```
    pub fn load_variables() -> Result<Self,LoadVariablesError>
    {
        let token = std::env::var("DISCORD_TOKEN").map_err(|_| LoadVariablesError::MissingRequiredEnvironmentVariable("DISCORD_TOKEN"))?;

        // We can silently handle a missing MAX_CONNECTIONS variable.
        // We just set it to a defauit (5).
        // If MAX_CONNECTIONS exists but isn't parseable we want to throw an error.
        let max_connections = std::env::var("MAX_CONNECTIONS");
        let max_connections = match max_connections
        {
            Ok(value) => value.parse::<u32>().map_err(|_| LoadVariablesError::EnvironmentVariableInInvalidFormat("MAX_CONNECTIONS","Couldn't parse MAX_CONNECTIONS as a u32"))?,
            Err(_) => 5,
        };

        let database_url = std::env::var("DATABASE_URL").map_err(|_| LoadVariablesError::MissingRequiredEnvironmentVariable("DATABASE_URL"))?;

        Ok(Self {
            token,
            max_connections,
            database_url
        })
    }

    pub fn token(&self) -> &str
    {
        &self.token
    }

    pub fn max_connections(&self) -> u32 
    {
        self.max_connections
    }
    
    pub fn database_url(&self) -> &str
    {
        &self.database_url
    }
}
