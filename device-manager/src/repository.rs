use crate::{Device, DeviceData, DeviceCommand, DeviceError, Result};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Device repository for database operations
#[derive(Clone)]
pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // DEVICE CRUD
    // ========================================================================

    pub async fn create_device(&self, device: &Device) -> Result<Device> {
        let device = sqlx::query_as::<_, Device>(
            r#"
            INSERT INTO devices (
                id, name, device_type, manufacturer, model, serial_number,
                location, status, config, metadata, created_at, updated_at,
                created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(&device.id)
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(&device.manufacturer)
        .bind(&device.model)
        .bind(&device.serial_number)
        .bind(&device.location)
        .bind(&device.status)
        .bind(&device.config)
        .bind(&device.metadata)
        .bind(&device.created_at)
        .bind(&device.updated_at)
        .bind(&device.created_by)
        .bind(&device.updated_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(device)
    }

    pub async fn get_device(&self, id: Uuid) -> Result<Device> {
        let device = sqlx::query_as::<_, Device>(
            "SELECT * FROM devices WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DeviceError::NotFound(format!("Device {} not found", id)))?;

        Ok(device)
    }

    pub async fn list_devices(
        &self,
        device_type: Option<String>,
        status: Option<String>,
        location_filter: Option<serde_json::Value>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Device>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT * FROM devices WHERE 1=1");

        if let Some(dt) = device_type {
            query.push(" AND device_type = ");
            query.push_bind(dt);
        }

        if let Some(st) = status {
            query.push(" AND status = ");
            query.push_bind(st);
        }

        if let Some(loc) = location_filter {
            query.push(" AND location @> ");
            query.push_bind(loc);
        }

        query.push(" ORDER BY created_at DESC");
        query.push(" LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let devices = query
            .build_query_as::<Device>()
            .fetch_all(&self.pool)
            .await?;

        Ok(devices)
    }

    pub async fn update_device(&self, id: Uuid, device: &Device) -> Result<Device> {
        let device = sqlx::query_as::<_, Device>(
            r#"
            UPDATE devices
            SET name = $2, device_type = $3, manufacturer = $4, model = $5,
                serial_number = $6, location = $7, status = $8, config = $9,
                metadata = $10, updated_at = $11, updated_by = $12
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(&device.manufacturer)
        .bind(&device.model)
        .bind(&device.serial_number)
        .bind(&device.location)
        .bind(&device.status)
        .bind(&device.config)
        .bind(&device.metadata)
        .bind(Utc::now())
        .bind(&device.updated_by)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DeviceError::NotFound(format!("Device {} not found", id)))?;

        Ok(device)
    }

    pub async fn update_device_status(
        &self,
        id: Uuid,
        status: String,
        error: Option<String>,
    ) -> Result<Device> {
        let device = sqlx::query_as::<_, Device>(
            r#"
            UPDATE devices
            SET status = $2,
                last_error = $3,
                last_connected = CASE WHEN $2 IN ('connected', 'active') THEN NOW() ELSE last_connected END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DeviceError::NotFound(format!("Device {} not found", id)))?;

        Ok(device)
    }

    pub async fn delete_device(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM devices WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DeviceError::NotFound(format!("Device {} not found", id)));
        }

        Ok(())
    }

    // ========================================================================
    // DEVICE DATA
    // ========================================================================

    pub async fn save_device_data(&self, data: &DeviceData) -> Result<DeviceData> {
        let data = sqlx::query_as::<_, DeviceData>(
            r#"
            INSERT INTO device_data (
                id, device_id, timestamp, data_type, format,
                raw_data, parsed_data, normalized_data,
                patient_id, encounter_id, provider_id, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(&data.id)
        .bind(&data.device_id)
        .bind(&data.timestamp)
        .bind(&data.data_type)
        .bind(&data.format)
        .bind(&data.raw_data)
        .bind(&data.parsed_data)
        .bind(&data.normalized_data)
        .bind(&data.patient_id)
        .bind(&data.encounter_id)
        .bind(&data.provider_id)
        .bind(&data.metadata)
        .bind(&data.created_at)
        .fetch_one(&self.pool)
        .await?;

        // Update device's last_data_received timestamp
        sqlx::query("UPDATE devices SET last_data_received = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(&data.device_id)
            .execute(&self.pool)
            .await?;

        Ok(data)
    }

    pub async fn get_device_data(
        &self,
        device_id: Uuid,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        data_type: Option<String>,
        limit: i64,
    ) -> Result<Vec<DeviceData>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT * FROM device_data WHERE device_id = "
        );
        query.push_bind(device_id);

        if let Some(start) = start_time {
            query.push(" AND timestamp >= ");
            query.push_bind(start);
        }

        if let Some(end) = end_time {
            query.push(" AND timestamp <= ");
            query.push_bind(end);
        }

        if let Some(dt) = data_type {
            query.push(" AND data_type = ");
            query.push_bind(dt);
        }

        query.push(" ORDER BY timestamp DESC LIMIT ");
        query.push_bind(limit);

        let data = query
            .build_query_as::<DeviceData>()
            .fetch_all(&self.pool)
            .await?;

        Ok(data)
    }

    // ========================================================================
    // DEVICE COMMANDS
    // ========================================================================

    pub async fn save_device_command(&self, command: &DeviceCommand) -> Result<DeviceCommand> {
        let command = sqlx::query_as::<_, DeviceCommand>(
            r#"
            INSERT INTO device_commands (
                id, device_id, command, parameters, metadata,
                created_at, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&command.id)
        .bind(&command.device_id)
        .bind(&command.command)
        .bind(&command.parameters)
        .bind(&command.metadata)
        .bind(&command.created_at)
        .bind(&command.status)
        .fetch_one(&self.pool)
        .await?;

        Ok(command)
    }

    pub async fn update_command_status(
        &self,
        id: Uuid,
        status: String,
        response: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<DeviceCommand> {
        let command = sqlx::query_as::<_, DeviceCommand>(
            r#"
            UPDATE device_commands
            SET status = $2,
                response = $3,
                error = $4,
                executed_at = CASE WHEN $2 = 'executing' THEN NOW() ELSE executed_at END,
                completed_at = CASE WHEN $2 IN ('completed', 'failed') THEN NOW() ELSE completed_at END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(response)
        .bind(error)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DeviceError::NotFound(format!("Command {} not found", id)))?;

        Ok(command)
    }

    pub async fn get_device_commands(
        &self,
        device_id: Uuid,
        status: Option<String>,
        limit: i64,
    ) -> Result<Vec<DeviceCommand>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT * FROM device_commands WHERE device_id = "
        );
        query.push_bind(device_id);

        if let Some(st) = status {
            query.push(" AND status = ");
            query.push_bind(st);
        }

        query.push(" ORDER BY created_at DESC LIMIT ");
        query.push_bind(limit);

        let commands = query
            .build_query_as::<DeviceCommand>()
            .fetch_all(&self.pool)
            .await?;

        Ok(commands)
    }
}
