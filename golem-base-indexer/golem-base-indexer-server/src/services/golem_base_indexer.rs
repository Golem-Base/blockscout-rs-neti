use crate::proto::{
    golem_base_indexer_service_server::GolemBaseIndexerService as GolemBaseIndexer, *,
};
use golem_base_indexer_logic::{
    repository,
    services::{BlockscoutService, RpcService},
    types::{ConsensusInfo, ListOperationsFilter, OperationType, OperationsFilter},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct ExternalServices {
    pub l2_blockscout: Arc<BlockscoutService>,
    pub l3_rpc: Arc<RpcService>,
}

pub struct GolemBaseIndexerService {
    db: Arc<DatabaseConnection>,
    services: ExternalServices,
}

impl GolemBaseIndexerService {
    pub fn new(db: Arc<DatabaseConnection>, services: ExternalServices) -> Self {
        Self { db, services }
    }
}

#[async_trait::async_trait]
impl GolemBaseIndexer for GolemBaseIndexerService {
    async fn get_entity(
        &self,
        request: Request<GetEntityRequest>,
    ) -> Result<Response<FullEntity>, Status> {
        let inner = request.into_inner();

        let key = inner
            .key
            .parse()
            .map_err(|_| Status::invalid_argument("Invalid entity key"))?;

        let entity = repository::entities::get_full_entity(&*self.db, key)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity");
                Status::internal(format!("failed to query entity - {err}"))
            })?
            .ok_or(Status::not_found("entity not found"))?;

        let string_attributes =
            repository::attributes::find_active_string_attributes(&*self.db, key)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query attributes");
                    Status::internal("failed to query attributes")
                })?;

        let numeric_attributes =
            repository::attributes::find_active_numeric_attributes(&*self.db, key)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query attributes");
                    Status::internal("failed to query attributes")
                })?;

        let entity = FullEntity::new(entity, string_attributes, numeric_attributes);

        Ok(Response::new(entity))
    }

    async fn get_operation(
        &self,
        request: Request<GetOperationRequest>,
    ) -> Result<Response<v1::EntityHistoryEntry>, Status> {
        let inner = request.into_inner();

        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entity operation filter: {err}"))
        })?;

        let operation = repository::entities::get_entity_operation(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity operation");
                Status::internal("failed to query entity operation")
            })?
            .ok_or(Status::not_found("operation not found"))?;

        Ok(Response::new(operation.into()))
    }

    async fn list_entities(
        &self,
        request: Request<ListEntitiesRequest>,
    ) -> Result<Response<ListEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            tracing::error!(?err, "Invalid filter");
            Status::invalid_argument("Invalid filter")
        })?;
        let (entities, pagination) = repository::entities::list_entities(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities");
                Status::internal("failed to query entities")
            })?;

        let items = entities.into_iter().map(Into::into).collect();
        let pagination = pagination.into();

        Ok(Response::new(ListEntitiesResponse {
            items,
            pagination: Some(pagination),
        }))
    }

    async fn count_entities(
        &self,
        request: Request<CountEntitiesRequest>,
    ) -> Result<Response<CountEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            tracing::error!(?err, "Invalid filter");
            Status::invalid_argument("Invalid filter")
        })?;
        let count = repository::entities::count_entities(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities");
                Status::internal("failed to query entities")
            })?;

        Ok(Response::new(CountEntitiesResponse { count }))
    }

    async fn list_operations(
        &self,
        request: Request<ListOperationsRequest>,
    ) -> Result<Response<ListOperationsResponse>, Status> {
        let inner = request.into_inner();
        let filter: ListOperationsFilter = inner
            .try_into()
            .map_err(|err| Status::invalid_argument(format!("Invalid operations filter: {err}")))?;

        let (operations, pagination) = repository::operations::list_operations(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query operations");
                Status::internal("failed to query operations")
            })?;

        let items = operations.into_iter().map(Into::into).collect();
        let pagination = pagination.into();

        Ok(Response::new(ListOperationsResponse {
            items,
            // for some reason, the protobuf forces pagination to be an option
            pagination: Some(pagination),
        }))
    }

    async fn count_operations(
        &self,
        request: Request<CountOperationsRequest>,
    ) -> Result<Response<CountOperationsResponse>, Status> {
        let inner = request.into_inner();
        let filter: OperationsFilter = inner
            .try_into()
            .map_err(|e| Status::invalid_argument(format!("Invalid operations filter: {e}")))?;

        let operations_count = repository::operations::count_operations(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to count operations");
                Status::internal("failed to count operations")
            })?;

        let operations_count = operations_count.into();

        Ok(Response::new(operations_count))
    }

    async fn get_entity_history(
        &self,
        request: Request<GetEntityHistoryRequest>,
    ) -> Result<Response<GetEntityHistoryResponse>, Status> {
        let inner = request.into_inner();

        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entity history filter: {err}"))
        })?;

        let (items, pagination) = repository::entities::get_entity_history(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity history");
                Status::internal("failed to query entity history")
            })?;

        Ok(Response::new(GetEntityHistoryResponse {
            items: items.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn address_stats(
        &self,
        request: Request<AddressStatsRequest>,
    ) -> Result<Response<AddressStatsResponse>, Status> {
        let AddressStatsRequest { address } = request.into_inner();
        let address = address.parse().map_err(|err| {
            tracing::error!(?err, "invalid address");
            Status::invalid_argument("invalid address")
        })?;

        let filter = OperationsFilter {
            sender: Some(address),
            ..Default::default()
        };

        let (entities_counts, tx_counts, operations_counts, address_activity) = tokio::join!(
            repository::address::count_entities(&*self.db, address),
            repository::address::count_txs(&*self.db, address),
            repository::operations::count_operations(&*self.db, filter),
            repository::address::get_address_activity(&*self.db, address)
        );
        let entities_counts = entities_counts.map_err(|err| {
            tracing::error!(?err, "failed to count entities");
            Status::internal("failed to count entities")
        })?;

        let tx_counts = tx_counts.map_err(|err| {
            tracing::error!(?err, "failed to count txs");
            Status::internal("failed to count txs")
        })?;

        let operations_counts = operations_counts.map_err(|err| {
            tracing::error!(?err, "failed to count operations");
            Status::internal("failed to count operations")
        })?;

        let address_activity = address_activity.map_err(|err| {
            tracing::error!(?err, "failed to get address activity");
            Status::internal("failed to get address activity")
        })?;

        Ok(Response::new(AddressStatsResponse {
            created_entities: entities_counts.created_entities,
            owned_entities: entities_counts.owned_entities,
            active_entities: entities_counts.active_entities,
            size_of_active_entities: entities_counts.size_of_active_entities,
            total_transactions: tx_counts.total_transactions,
            failed_transactions: tx_counts.failed_transactions,
            operations_counts: Some(operations_counts.into()),
            first_seen_timestamp: address_activity
                .first_seen_timestamp
                .map(|v| v.to_rfc3339()),
            last_seen_timestamp: address_activity.last_seen_timestamp.map(|v| v.to_rfc3339()),
            first_seen_block: address_activity.first_seen_block,
            last_seen_block: address_activity.last_seen_block,
        }))
    }

    async fn block_stats(
        &self,
        request: Request<BlockStatsRequest>,
    ) -> Result<Response<BlockStatsResponse>, Status> {
        let BlockStatsRequest { block_number } = request.into_inner();
        let block_number = block_number.parse().map_err(|err| {
            tracing::error!(?err, "invalid block number");
            Status::invalid_argument("invalid block number")
        })?;

        // Get entity counts
        let counts = repository::block::count_entities(&*self.db, block_number)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query block stats counts");
                Status::internal("failed to query block stats counts")
            })?;

        // Get storage usage
        let storage = repository::block::storage_usage(&*self.db, block_number)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query block storage usage");
                Status::internal("failed to query block storage usage")
            })?;

        Ok(Response::new(BlockStatsResponse {
            counts: Some(counts.into()),
            storage: Some(storage.into()),
        }))
    }

    async fn list_custom_contract_transactions(
        &self,
        request: Request<ListCustomContractTransactionsRequest>,
    ) -> Result<Response<ListCustomContractTransactionsResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!(
                "Invalid custom contract transactions filter: {err}"
            ))
        })?;

        let (transactions, pagination) =
            repository::transactions::list_custom_contract_transactions(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query custom contract transactions");
                    Status::internal("failed to query custom contract transactions")
                })?;

        Ok(Response::new(ListCustomContractTransactionsResponse {
            items: transactions.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn entities_averages(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<EntitiesAveragesResponse>, Status> {
        let entities_averages = repository::entities::entities_averages(&*self.db)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to get entities averages");
                Status::internal("failed to get entities averages")
            })?;

        Ok(Response::new(entities_averages.into()))
    }

    async fn address_leaderboard_ranks(
        &self,
        request: Request<AddressLeaderboardRanksRequest>,
    ) -> Result<Response<AddressLeaderboardRanksResponse>, Status> {
        let AddressLeaderboardRanksRequest { address } = request.into_inner();
        let address = address.parse().map_err(|err| {
            tracing::error!(?err, "invalid address");
            Status::invalid_argument("invalid address")
        })?;

        let leaderboard_ranks =
            repository::address::get_address_leaderboard_ranks(&*self.db, address)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to get address leaderboard ranks");
                    Status::internal("failed to get address leaderboard ranks")
                })?;

        Ok(Response::new(leaderboard_ranks.into()))
    }

    // Charts
    async fn chart_data_usage(
        &self,
        request: Request<ChartDataUsageRequest>,
    ) -> Result<Response<ChartResponse>, Status> {
        let inner = request.into_inner();
        let resolution = inner
            .resolution
            .try_into()
            .map_err(|_| Status::invalid_argument("Unsupported chart resolution"))?;
        let (points, info) = repository::timeseries::data_usage::timeseries_data_usage(
            &*self.db, inner.from, inner.to, resolution,
        )
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to query data usage chart");
            Status::internal("failed to query data usage chart")
        })?;

        Ok(Response::new(ChartResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn chart_storage_forecast(
        &self,
        request: Request<ChartStorageForecastRequest>,
    ) -> Result<Response<ChartResponse>, Status> {
        let inner = request.into_inner();
        let resolution = inner
            .resolution
            .try_into()
            .map_err(|_| Status::invalid_argument("Unsupported chart resolution"))?;

        let (points, info) = repository::timeseries::storage_forecast::timeseries_storage_forecast(
            &*self.db, &inner.to, resolution,
        )
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to query storage forecast chart");
            Status::internal("failed to query storage forecast chart")
        })?;

        Ok(Response::new(ChartResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn get_entity_data_histogram(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<GetEntityDataHistogramResponse>, Status> {
        let entity_data_size_histogram =
            repository::entities::get_entity_size_data_histogram(&*self.db)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query entity data histogram");
                    Status::internal("failed to query entity data histogram")
                })?;

        Ok(Response::new(GetEntityDataHistogramResponse {
            items: entity_data_size_histogram
                .into_iter()
                .map(Into::into)
                .collect(),
        }))
    }
    async fn chart_operation_count(
        &self,
        request: Request<ChartOperationCountRequest>,
    ) -> Result<Response<ChartOperationCountResponse>, Status> {
        let inner = request.into_inner();
        let resolution = inner
            .resolution
            .try_into()
            .map_err(|_| Status::invalid_argument("Unsupported chart resolution"))?;
        let operation: operation_type_filter::OperationTypeFilter = inner
            .operation
            .try_into()
            .map_err(|_| Status::invalid_argument("Invalid operation filter"))?;
        let operation: Option<OperationType> = operation.into();
        let (points, info) = repository::timeseries::operation_count::timeseries_operation_count(
            &*self.db, inner.from, inner.to, resolution, operation,
        )
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to query operation count timeseries");
            Status::internal("failed to query operation count timeseries")
        })?;

        Ok(Response::new(ChartOperationCountResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn chart_entity_count(
        &self,
        request: Request<ChartEntityCountRequest>,
    ) -> Result<Response<ChartResponse>, Status> {
        let inner = request.into_inner();
        let resolution = inner
            .resolution
            .try_into()
            .map_err(|_| Status::invalid_argument("Unsupported chart resolution"))?;
        let (points, info) = repository::timeseries::entity_count::timeseries_entity_count(
            &*self.db, inner.from, inner.to, resolution,
        )
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to query entity count chart");
            Status::internal("failed to query entity count chart")
        })?;

        Ok(Response::new(ChartResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn chart_block_transactions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ChartBlockTransactionsResponse>, Status> {
        let (points, info) =
            repository::timeseries::block_transactions::timeseries_block_transactions(&*self.db)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query block transactions timeseries");
                    Status::internal("failed to query block transactions timeseries")
                })?;

        Ok(Response::new(ChartBlockTransactionsResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn chart_block_operations(
        &self,
        request: Request<ChartBlockOperationsRequest>,
    ) -> Result<Response<ChartBlockOperationsResponse>, Status> {
        let inner = request.into_inner();
        let limit = inner.limit.unwrap_or(100).clamp(1, 500);

        let (points, info) =
            repository::timeseries::block_operations::timeseries_block_operations(&*self.db, limit)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query block operations timeseries");
                    Status::internal("failed to query block operations timeseries")
                })?;

        Ok(Response::new(ChartBlockOperationsResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    async fn chart_block_gas_usage_limit(
        &self,
        request: Request<ChartBlockGasUsageLimitRequest>,
    ) -> Result<Response<ChartBlockGasUsageLimitResponse>, Status> {
        let inner = request.into_inner();
        let limit = inner.limit.unwrap_or(1800);

        let (points, info) =
            repository::timeseries::block_gas_usage_limit::timeseries_block_gas_usage_limit(
                &*self.db, limit,
            )
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query block gas usage and limit timeseries");
                Status::internal("failed to query block gas usage and limit timeseries")
            })?;

        Ok(Response::new(ChartBlockGasUsageLimitResponse {
            chart: points.into_iter().map(Into::into).collect(),
            info: Some(info.into()),
        }))
    }

    // Leaderboards
    async fn leaderboard_biggest_spenders(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardBiggestSpendersResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid biggest spenders filter: {err}"))
        })?;

        let (spenders, pagination) =
            repository::leaderboards::leaderboard_biggest_spenders(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query biggest spenders");
                    Status::internal("failed to query biggest spenders")
                })?;

        Ok(Response::new(LeaderboardBiggestSpendersResponse {
            items: spenders.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_top_accounts(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardTopAccountsResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid top accounts filter: {err}"))
        })?;

        let (top_accounts, pagination) =
            repository::leaderboards::leaderboard_top_accounts(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by top accounts");
                    Status::internal("failed to query addresses by top accounts")
                })?;

        Ok(Response::new(LeaderboardTopAccountsResponse {
            items: top_accounts.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_entities_created(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardEntitiesCreatedResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities created filter: {err}"))
        })?;

        let (entities_created, pagination) =
            repository::leaderboards::leaderboard_entities_created(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by entities created");
                    Status::internal("failed to query addresses by entities created")
                })?;

        Ok(Response::new(LeaderboardEntitiesCreatedResponse {
            items: entities_created.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_entities_owned(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardEntitiesOwnedResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities owned filter: {err}"))
        })?;

        let (entities_owned, pagination) =
            repository::leaderboards::leaderboard_entities_owned(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by entities owned");
                    Status::internal("failed to query addresses by entities owned")
                })?;

        Ok(Response::new(LeaderboardEntitiesOwnedResponse {
            items: entities_owned.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_data_owned(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardDataOwnedResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities owned filter: {err}"))
        })?;

        let (data_owned, pagination) =
            repository::leaderboards::leaderboard_data_owned(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by data owned");
                    Status::internal("failed to query addresses by data owned")
                })?;

        Ok(Response::new(LeaderboardDataOwnedResponse {
            items: data_owned.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_largest_entities(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardLargestEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid largest entities filter: {err}"))
        })?;

        let (largest_entities, pagination) =
            repository::leaderboards::leaderboard_largest_entities(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query largest entities");
                    Status::internal("failed to query largest entities")
                })?;

        Ok(Response::new(LeaderboardLargestEntitiesResponse {
            items: largest_entities.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn leaderboard_effectively_largest_entities(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardEffectivelyLargestEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!(
                "Invalid effectively largest entities filter: {err}"
            ))
        })?;

        let (largest_entities, pagination) =
            repository::leaderboards::leaderboard_effectively_largest_entities(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query effectively largest entities");
                    Status::internal("failed to query effectively largest entities")
                })?;

        Ok(Response::new(
            LeaderboardEffectivelyLargestEntitiesResponse {
                items: largest_entities.into_iter().map(Into::into).collect(),
                pagination: Some(pagination.into()),
            },
        ))
    }

    async fn leaderboard_entities_by_btl(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<LeaderboardEntitiesByBtlResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities by btl filter: {err}"))
        })?;

        let (entities, pagination) =
            repository::leaderboards::leaderboard_entities_by_btl(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query entities by btl");
                    Status::internal("failed to query entities by btl")
                })?;

        Ok(Response::new(LeaderboardEntitiesByBtlResponse {
            items: entities.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn get_consensus_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ConsensusInfoResponse>, Status> {
        let (blocks_result, gas_result) = tokio::join!(
            self.services.l3_rpc.get_consensus_blocks_info_cached(),
            self.services.l2_blockscout.get_consensus_gas_info_cached()
        );

        Ok(Response::new(
            ConsensusInfo {
                blocks: blocks_result.unwrap_or_default(),
                gas: gas_result.unwrap_or_default(),
            }
            .into(),
        ))
    }
}
