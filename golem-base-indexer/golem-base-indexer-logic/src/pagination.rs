use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, Paginator, SelectorTrait};

use crate::types::PaginationMetadata;

/// Paginate items with metadata
/// C - connection
/// S - selector (items)
/// D - destination type
pub async fn paginate_try_from<'a, C, S, D>(
    paginator: Paginator<'a, C, S>,
    page: u64,
    page_size: u64,
) -> Result<(Vec<D>, PaginationMetadata)>
where
    C: ConnectionTrait,
    S: SelectorTrait,
    D: TryFrom<S::Item>,
    <D as TryFrom<S::Item>>::Error: Into<anyhow::Error>,
{
    let total_items = paginator
        .num_items()
        .await
        .context("Failed to count items")?;
    let total_pages = paginator
        .num_pages()
        .await
        .context("Failed to get number of pages")?;
    let page_index = page.saturating_sub(1);
    let items = paginator
        .fetch_page(page_index)
        .await
        .context("Failed to fetch page")?
        .into_iter()
        .map(|item| D::try_from(item).map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;

    let pagination_metadata = PaginationMetadata {
        page,
        page_size,
        total_pages,
        total_items,
    };
    Ok((items, pagination_metadata))
}
