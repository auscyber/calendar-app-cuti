// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, sync::Arc};

use dotenv::dotenv;
use notion::{
    ids::{AsIdentifier, DatabaseId, PageId, PropertyId},
    models::{
        paging::Paging,
        properties::{Color, DateValue, PropertyConfiguration, PropertyValue, SelectOptionId},
        search::{DatabaseQuery, Filter, FilterCondition, NotionSearch, PropertyCondition},
        text::RichText,
        Database, DateTime, IconObject, Object, Page, Utc,
    },
    NotionApi,
};

use serde::{Deserialize, Serialize};
use serde_json::error;
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug)]
struct DatabaseDisplay {
    id: DatabaseId,
    icon: Option<IconObject>,
    title: String,
    properties: HashMap<String, PropertyConfiguration>,
}

impl AsIdentifier<DatabaseId> for &DatabaseDisplay {
    fn as_id(&self) -> &DatabaseId {
        &self.id
    }
}

fn display_database(database: &Database) -> DatabaseDisplay {
    DatabaseDisplay {
        id: database.id.clone(),
        icon: database.icon.clone(),
        title: database.title_plain_text(),
        properties: database
            .properties
            .iter()
            .filter(|(_, v)| match v {
                PropertyConfiguration::Date { .. } => true,
                PropertyConfiguration::Title { .. } => true,
                PropertyConfiguration::Select { .. } => true,
                PropertyConfiguration::MultiSelect { .. } => true,
                PropertyConfiguration::Relation { .. } => true,
                PropertyConfiguration::Status { .. } => true,
                _ => false,
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    }
}

struct DatabaseConfig {
    id: DatabaseId,
    nameObj: PropertyId,
    dateObj: PropertyId,
    statusObj: PropertyId,
    extra_properties: HashMap<String, PropertyConfiguration>,
}

#[derive(Debug, TS, Serialize)]
struct SelectOption {
    #[ts(type = "String")]
    id: SelectOptionId,
    name: String,
    #[ts(as = "String")]
    color: Color,
}

#[derive(Debug, TS, Serialize)]
struct RelationLink {
    #[ts(type = "string")]
    id: PageId,
    name: Option<String>,
    #[ts(
        type = "null | import(\"@notionhq/client/build/src/api-endpoints\").PageObjectResponse[\"icon\"]"
    )]
    icon: Option<IconObject>,
}

#[derive(Debug, TS, Serialize)]
#[ts(export)]
pub(crate) struct Task {
    #[ts(type = "string")]
    id: PageId,
    name: String,
    #[ts(type = "Date")]
    due_date: DateValue,
    status: SelectOption,
    class: Vec<RelationLink>,
    type_: Vec<SelectOption>,
    #[ts(type = "object")]
    extra_props: HashMap<String, PropertyValue>,
}

async fn page_to_task(api: &NotionApi, page: &Page) -> Result<Task, Error> {
    let name = page
        .properties
        .properties
        .get("Name")
        .and_then(|x| {
            if let PropertyValue::Title { id, title } = x {
                Some(
                    title
                        .into_iter()
                        .map(|x| x.plain_text())
                        .collect::<String>(),
                )
            } else {
                None
            }
        })
        .unwrap();
    let due_date = page
        .properties
        .properties
        .get("Due Date")
        .and_then(|x| {
            if let PropertyValue::Date { id, date } = x {
                date.clone()
            } else {
                None
            }
        })
        .unwrap();
    let status = page
        .properties
        .properties
        .get("Status")
        .and_then(|x| {
            if let PropertyValue::Status { id, status } = x {
                Some(SelectOption {
                    id: status.as_ref()?.id.clone()?,
                    name: status.as_ref()?.name.clone()?,
                    color: status.as_ref()?.color.clone(),
                })
            } else {
                None
            }
        })
        .unwrap();
    let class = page
        .properties
        .properties
        .get("Class")
        .map(|x| async move {
            if let PropertyValue::Relation { id, relation } = x {
                Some(
                    futures::future::join_all(relation.as_ref()?.iter().map(|x| async {
                        let page = api.get_page(x.id.clone()).await?;
                        Ok(RelationLink {
                            id: x.id.clone(),
                            name: page.title(),
                            icon: page.icon.clone(),
                        })
                    }))
                    .await,
                )
            } else {
                None
            }
        })
        .unwrap()
        .await
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>, Error>>()?;
    let type_ = page
        .properties
        .properties
        .get("Type")
        .and_then(|x| {
            if let PropertyValue::MultiSelect { id, multi_select } = x {
                Some(
                    multi_select
                        .clone()
                        .as_ref()?
                        .iter()
                        .filter_map(|x| {
                            Some(SelectOption {
                                id: x.id.clone()?,
                                name: x.name.clone()?,
                                color: x.color.clone(),
                            })
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap();
    //    let extra_props = page
    //        .properties
    //        .properties
    //        .iter()
    //        .filter_map(|(k, v)| {
    //            if !["Name", "Due Date", "Status"].contains(&k.as_str()) {
    //                Some((k.clone(), v.clone()))
    //            } else {
    //                None
    //            }
    //        })
    //        .collect();
    Ok(Task {
        id: page.id.clone(),
        name: name,
        due_date: due_date,
        status: status,
        class: class,
        type_: type_,
        extra_props: HashMap::new(),
    })
}

#[derive(Debug, Error, TS)]
#[ts(export)]
#[ts(as = "String")]
enum Error {
    #[error("Notion error: {0}")]
    Notion(#[from] notion::Error),
    #[error("Environment error: {0}")]
    VarError(#[from] std::env::VarError),
    #[error("Dotenv error: {0}")]
    Dotenv(#[from] dotenv::Error),
}
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{}", self);
        s.serialize(serializer)
    }
}

#[tauri::command]
async fn tasks() -> Result<Vec<Task>, Error> {
    dotenv()?;

    let notion_api_key = std::env::var("NOTION_API_KEY")?;
    let api = Arc::new(notion::NotionApi::new(notion_api_key)?);
    let databases = api.search(NotionSearch::filter_by_databases()).await?;
    let databases = databases
        .only_databases()
        .results()
        .into_iter()
        .map(display_database)
        .collect::<Vec<_>>();
    let first_database = databases.first().unwrap();
    let first_10 = api
        .query_database(
            first_database,
            DatabaseQuery {
                filter: Some(notion::models::search::FilterCondition::And {
                    and: vec![FilterCondition::Property {
                        property: first_database
                            .properties
                            .iter()
                            .find_map(|(k, p)| match p {
                                PropertyConfiguration::Date { id } if k == "Due Date" => Some(id),
                                _ => None,
                            })
                            .unwrap()
                            .clone()
                            .to_string(),
                        condition: PropertyCondition::Date(
                            notion::models::search::DateCondition::OnOrAfter(Utc::now()),
                        ),
                    }],
                }),
                paging: Some(Paging {
                    page_size: Some(10),
                    start_cursor: None,
                }),
                ..DatabaseQuery::default()
            },
        )
        .await?;
    let all_tasks = first_10
        .results()
        .iter()
        .map(|page| {
            let api = api.clone();
            async move { page_to_task(&api, &page).await }
        })
        .collect::<Vec<_>>();

    let res = futures::future::join_all(all_tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, Error>>()?;
    Ok::<_, Error>(res)

    // Your code here
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![tasks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
