use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct GraphQLRequest {
    query: String,
    variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PackageData {
    address: String,
    version: u64,
    #[serde(rename = "packageVersions")]
    package_versions: Option<PackageVersions>,
}

#[derive(Debug, Deserialize)]
struct PackageVersions {
    nodes: Vec<PackageVersionNode>,
}

#[derive(Debug, Deserialize)]
struct PackageVersionNode {
    address: String,
    version: u64,
    #[serde(rename = "packageBcs")]
    package_bcs: Option<String>,
    #[serde(rename = "moduleBcs")]
    module_bcs: Option<Vec<String>>,
}

async fn query_sui_package(package_address: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    // The GraphQL query
    let query = r#"
        query PackageQuery($address: SuiAddress!) {
            package(address: $address) {
                address
                version
                packageVersions(first: 50) {
                    nodes {
                        address
                        version
                        packageBcs
                    }
                }
            }
        }
    "#;
    
    // Create variables
    let mut variables = HashMap::new();
    variables.insert("address".to_string(), json!(package_address));
    
    // Create the request
    let request = GraphQLRequest {
        query: query.to_string(),
        variables,
    };
    
    // Send the request to Sui mainnet GraphQL endpoint
    let response = client
        .post("https://sui-mainnet.mystenlabs.com/graphql")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    // Check if the request was successful
    if !response.status().is_success() {
        println!("HTTP Error: {}", response.status());
        let error_text = response.text().await?;
        println!("Error response: {}", error_text);
        return Ok(());
    }
    
    // Parse the response
    let graphql_response: GraphQLResponse = response.json().await?;
    
    // Handle errors
    if let Some(errors) = graphql_response.errors {
        println!("GraphQL Errors:");
        for error in errors {
            println!("  - {}", error.message);
        }
        return Ok(());
    }
    
    // Process the data
    if let Some(data) = graphql_response.data {
        if let Some(package_data) = data.get("package") {
            if package_data.is_null() {
                println!("Package not found at address: {}", package_address);
                return Ok(());
            }

            let response_json = json!({
                "status": "success",
                "query": {
                    "package_address": package_address,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "endpoint": "https://sui-mainnet.mystenlabs.com/graphql"
                },
                "data": data
            });
            
            std::fs::write("response.json", serde_json::to_string_pretty(&response_json)?)?;
            
            // Try to deserialize the package data
            match serde_json::from_value::<PackageData>(package_data.clone()) {
                Ok(package) => {
                    println!("=== Package Information ===");
                    println!("Address: {}", package.address);
                    println!("Current Version: {}", package.version);
                    
                    if let Some(versions) = package.package_versions {
                        println!("\n=== Package Versions ({} found) ===", versions.nodes.len());
                        
                        for (i, version) in versions.nodes.iter().enumerate() {
                            println!("\n--- Version {} ---", i + 1);
                            println!("  Address: {}", version.address);
                            println!("  Version Number: {}", version.version);
                            
                            if let Some(package_bcs) = &version.package_bcs {
                                println!("  Package BCS Length: {} characters", package_bcs.len());
                                // Show first 100 characters of BCS data
                                if package_bcs.len() > 100 {
                                    println!("  Package BCS Preview: {}...", &package_bcs[..100]);
                                } else {
                                    println!("  Package BCS: {}", package_bcs);
                                }
                            } else {
                                println!("  Package BCS: None");
                            }
                            
                            if let Some(modules) = &version.module_bcs {
                                println!("  Number of Modules: {}", modules.len());
                                for (j, module) in modules.iter().enumerate() {
                                    println!("    Module {}: {} characters", j + 1, module.len());
                                    // Show first 50 characters of each module
                                    if module.len() > 50 {
                                        println!("      Preview: {}...", &module[..50]);
                                    }
                                }
                            } else {
                                println!("  Module BCS: None");
                            }
                        }
                    } else {
                        println!("No package versions found");
                    }
                }
                Err(e) => {
                    println!("Failed to parse package data: {}", e);
                    println!("Raw data: {}", serde_json::to_string_pretty(package_data)?);
                }
            }
        } else {
            println!("No package data in response");
            println!("Full response: {}", serde_json::to_string_pretty(&data)?);
        }
    } else {
        println!("No data in response");
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Querying Sui Package...");
    
    // The package address you want to query
    let package_address = "0xc33c3e937e5aa2009cc0c3fdb3f345a0c3193d4ee663ffc601fe8b894fbc4ba6";
    
    match query_sui_package(package_address).await {
        Ok(_) => println!("\nQuery completed successfully!"),
        Err(e) => println!("Error occurred: {}", e),
    }
    
    Ok(())
}