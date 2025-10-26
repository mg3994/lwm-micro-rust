use sqlx::PgPool;
use uuid::Uuid;

use linkwithmentor_common::AppError;
use crate::models::{Dashboard, DashboardWidget, WidgetType, WidgetPosition, WidgetSize};
use crate::metrics::MetricsService;

#[derive(Clone)]
pub struct DashboardService {
    db_pool: PgPool,
    metrics_service: MetricsService,
}

impl DashboardService {
    pub fn new(db_pool: PgPool, metrics_service: MetricsService) -> Self {
        Self {
            db_pool,
            metrics_service,
        }
    }

    pub async fn create_dashboard(&self, user_id: Uuid, name: String, description: Option<String>) -> Result<Dashboard, AppError> {
        let dashboard_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query!(
            "INSERT INTO analytics_dashboards (dashboard_id, name, description, user_id, widgets, is_public, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            dashboard_id,
            name,
            description,
            user_id,
            serde_json::Value::Array(vec![]),
            false,
            now,
            now
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Dashboard {
            dashboard_id,
            name,
            description,
            user_id,
            widgets: Vec::new(),
            is_public: false,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_dashboard(&self, dashboard_id: Uuid, user_id: Uuid) -> Result<Dashboard, AppError> {
        let row = sqlx::query!(
            "SELECT dashboard_id, name, description, user_id, widgets, is_public, created_at, updated_at
             FROM analytics_dashboards 
             WHERE dashboard_id = $1 AND (user_id = $2 OR is_public = true)",
            dashboard_id,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Dashboard not found".to_string()))?;

        let widgets: Vec<DashboardWidget> = serde_json::from_value(row.widgets)
            .map_err(|e| AppError::Internal(format!("Failed to parse widgets: {}", e)))?;

        Ok(Dashboard {
            dashboard_id: row.dashboard_id,
            name: row.name,
            description: row.description,
            user_id: row.user_id,
            widgets,
            is_public: row.is_public,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn list_dashboards(&self, user_id: Uuid) -> Result<Vec<Dashboard>, AppError> {
        let rows = sqlx::query!(
            "SELECT dashboard_id, name, description, user_id, widgets, is_public, created_at, updated_at
             FROM analytics_dashboards 
             WHERE user_id = $1 OR is_public = true
             ORDER BY updated_at DESC",
            user_id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut dashboards = Vec::new();
        for row in rows {
            let widgets: Vec<DashboardWidget> = serde_json::from_value(row.widgets)
                .map_err(|e| AppError::Internal(format!("Failed to parse widgets: {}", e)))?;

            dashboards.push(Dashboard {
                dashboard_id: row.dashboard_id,
                name: row.name,
                description: row.description,
                user_id: row.user_id,
                widgets,
                is_public: row.is_public,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(dashboards)
    }

    pub async fn add_widget(&self, dashboard_id: Uuid, user_id: Uuid, widget: DashboardWidget) -> Result<(), AppError> {
        // First, get the current dashboard
        let mut dashboard = self.get_dashboard(dashboard_id, user_id).await?;
        
        // Check if user owns the dashboard
        if dashboard.user_id != user_id {
            return Err(AppError::Forbidden("Cannot modify dashboard".to_string()));
        }

        // Add the new widget
        dashboard.widgets.push(widget);
        dashboard.updated_at = chrono::Utc::now();

        // Update the dashboard
        sqlx::query!(
            "UPDATE analytics_dashboards 
             SET widgets = $1, updated_at = $2 
             WHERE dashboard_id = $3 AND user_id = $4",
            serde_json::to_value(&dashboard.widgets).unwrap(),
            dashboard.updated_at,
            dashboard_id,
            user_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_widget(&self, dashboard_id: Uuid, user_id: Uuid, widget_id: Uuid) -> Result<(), AppError> {
        let mut dashboard = self.get_dashboard(dashboard_id, user_id).await?;
        
        if dashboard.user_id != user_id {
            return Err(AppError::Forbidden("Cannot modify dashboard".to_string()));
        }

        // Remove the widget
        dashboard.widgets.retain(|w| w.widget_id != widget_id);
        dashboard.updated_at = chrono::Utc::now();

        sqlx::query!(
            "UPDATE analytics_dashboards 
             SET widgets = $1, updated_at = $2 
             WHERE dashboard_id = $3 AND user_id = $4",
            serde_json::to_value(&dashboard.widgets).unwrap(),
            dashboard.updated_at,
            dashboard_id,
            user_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_default_dashboard(&self, user_id: Uuid) -> Result<Dashboard, AppError> {
        // Create a default dashboard with common widgets
        let dashboard_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let default_widgets = vec![
            DashboardWidget {
                widget_id: Uuid::new_v4(),
                widget_type: WidgetType::MetricCard,
                title: "Total Users".to_string(),
                configuration: serde_json::json!({
                    "metric": "total_users",
                    "format": "number"
                }),
                position: WidgetPosition { x: 0, y: 0 },
                size: WidgetSize { width: 4, height: 2 },
            },
            DashboardWidget {
                widget_id: Uuid::new_v4(),
                widget_type: WidgetType::LineChart,
                title: "Daily Active Users".to_string(),
                configuration: serde_json::json!({
                    "metric": "active_users_daily",
                    "time_range": "30d"
                }),
                position: WidgetPosition { x: 4, y: 0 },
                size: WidgetSize { width: 8, height: 4 },
            },
            DashboardWidget {
                widget_id: Uuid::new_v4(),
                widget_type: WidgetType::BarChart,
                title: "Revenue by Category".to_string(),
                configuration: serde_json::json!({
                    "metric": "revenue_by_category",
                    "time_range": "30d"
                }),
                position: WidgetPosition { x: 0, y: 4 },
                size: WidgetSize { width: 6, height: 4 },
            },
        ];

        Ok(Dashboard {
            dashboard_id,
            name: "Default Dashboard".to_string(),
            description: Some("Default analytics dashboard".to_string()),
            user_id,
            widgets: default_widgets,
            is_public: false,
            created_at: now,
            updated_at: now,
        })
    }
}