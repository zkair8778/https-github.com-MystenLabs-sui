// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use std::env;
use std::net::SocketAddr;

use hyper::header::HeaderValue;
use hyper::Method;
pub use jsonrpsee::server::ServerHandle;
use jsonrpsee::server::{AllowHosts, ServerBuilder};
use jsonrpsee::RpcModule;
use tower::util::option_layer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

use crate::metrics::MetricsLayer;
use sui_open_rpc::{Module, Project};

pub mod api;
pub mod bcs_api;
pub mod estimator_api;
pub mod event_api;
pub mod gateway_api;
mod metrics;
pub mod read_api;
pub mod streaming_api;
pub mod transaction_builder_api;
pub mod transaction_execution_api;

pub struct JsonRpcServerBuilder {
    module: RpcModule<()>,
    rpc_doc: Project,
    metrics: Option<MetricsLayer>,
}

pub fn sui_rpc_doc(version: &str) -> Project {
    Project::new(
        version,
        "Sui JSON-RPC",
        "Sui JSON-RPC API for interaction with Sui Full node.",
        "Mysten Labs",
        "https://mystenlabs.com",
        "build@mystenlabs.com",
        "Apache-2.0",
        "https://raw.githubusercontent.com/MystenLabs/sui/main/LICENSE",
    )
}

impl JsonRpcServerBuilder {
    pub fn new(version: &str, prometheus_registry: &prometheus::Registry) -> anyhow::Result<Self> {
        let metrics = MetricsLayer::new(prometheus_registry);
        Ok(Self {
            module: RpcModule::new(()),
            rpc_doc: sui_rpc_doc(version),
            metrics: Some(metrics),
        })
    }

    pub fn new_without_metrics_for_testing() -> anyhow::Result<Self> {
        Ok(Self {
            module: RpcModule::new(()),
            rpc_doc: sui_rpc_doc("0.0.0"),
            metrics: None,
        })
    }

    pub fn register_module<T: SuiRpcModule>(&mut self, module: T) -> Result<(), anyhow::Error> {
        self.rpc_doc.add_module(T::rpc_doc_module());
        Ok(self.module.merge(module.rpc())?)
    }

    pub async fn start(
        mut self,
        listen_address: SocketAddr,
    ) -> Result<ServerHandle, anyhow::Error> {
        let acl = match env::var("ACCESS_CONTROL_ALLOW_ORIGIN") {
            Ok(value) => {
                let allow_hosts = value
                    .split(',')
                    .into_iter()
                    .map(HeaderValue::from_str)
                    .collect::<Result<Vec<_>, _>>()?;
                AllowOrigin::list(allow_hosts)
            }
            _ => AllowOrigin::any(),
        };
        info!(?acl);

        let cors = CorsLayer::new()
            // Allow `POST` when accessing the resource
            .allow_methods([Method::POST])
            // Allow requests from any origin
            .allow_origin(acl)
            .allow_headers([hyper::header::CONTENT_TYPE]);

        let metrics_layer = option_layer(self.metrics);
        let middleware = tower::ServiceBuilder::new()
            .layer(cors)
            .layer(metrics_layer);

        let server = ServerBuilder::default()
            .set_host_filtering(AllowHosts::Any)
            .set_middleware(middleware)
            .build(listen_address)
            .await?;

        self.module
            .register_method("rpc.discover", move |_, _| Ok(self.rpc_doc.clone()))?;
        let methods_names = self.module.method_names().collect::<Vec<_>>();

        let addr = server.local_addr()?;
        let handle = server.start(self.module)?;

        info!(local_addr =? addr, "Sui JSON-RPC server listening on {addr}");
        info!("Available JSON-RPC methods : {:?}", methods_names);

        Ok(handle)
    }
}

pub trait SuiRpcModule
where
    Self: Sized,
{
    fn rpc(self) -> RpcModule<Self>;
    fn rpc_doc_module() -> Module;
}
