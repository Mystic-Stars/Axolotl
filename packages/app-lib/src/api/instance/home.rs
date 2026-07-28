use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::state::{
    DailyPlaytime, DailyPlaytimeEntry, InstanceMetadata, State,
};

pub async fn set_pinned(
    instance_id: &str,
    pinned: bool,
) -> crate::Result<InstanceMetadata> {
    let state = State::get().await?;
    crate::state::instances::commands::set_instance_pinned(
        instance_id,
        pinned,
        &state.pool,
    )
    .await?;

    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
                .as_error()
        })?;

    emit_instance(&instance.instance.id, InstancePayloadType::Edited).await?;

    Ok(instance)
}

pub async fn get_daily_playtime(
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> crate::Result<Vec<DailyPlaytime>> {
    let state = State::get().await?;
    crate::state::instances::commands::get_daily_playtime(
        start_date,
        end_date,
        &state.pool,
    )
    .await
}

pub async fn get_daily_playtime_details(
    date: chrono::NaiveDate,
) -> crate::Result<Vec<DailyPlaytimeEntry>> {
    let state = State::get().await?;
    crate::state::instances::commands::get_daily_playtime_details(
        date,
        &state.pool,
    )
    .await
}
