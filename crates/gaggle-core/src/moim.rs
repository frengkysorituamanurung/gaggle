//! Model-Owned Instruction Memory (MOIM).
//!
//! MOIM allows the model to own and manage instruction memory that persists
//! across sessions. The model can add, update, and remove instructions that
//! are automatically included in future context assembly.

use serde::{Deserialize, Serialize};

/// A model-owned instruction.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelInstruction {
    pub id: String,
    pub chat_id: String,
    pub instruction: String,
    pub priority: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// MOIM manager.
pub struct MoimManager {
    pool: sqlx::SqlitePool,
}

impl MoimManager {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Add a new instruction.
    pub async fn add_instruction(
        &self,
        chat_id: &str,
        instruction: &str,
        priority: i64,
    ) -> Result<ModelInstruction, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        sqlx::query(
            r#"
            INSERT INTO model_instructions (id, chat_id, instruction, priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(instruction)
        .bind(priority)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(ModelInstruction {
            id,
            chat_id: chat_id.to_string(),
            instruction: instruction.to_string(),
            priority,
            created_at: now,
            updated_at: now,
        })
    }

    /// List instructions for a chat.
    pub async fn list_instructions(
        &self,
        chat_id: &str,
    ) -> Result<Vec<ModelInstruction>, sqlx::Error> {
        sqlx::query_as::<_, ModelInstruction>(
            "SELECT * FROM model_instructions WHERE chat_id = ? ORDER BY priority DESC, created_at ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Remove an instruction.
    pub async fn remove_instruction(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM model_instructions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_instruction_serializes() {
        let instr = ModelInstruction {
            id: "1".to_string(),
            chat_id: "c1".to_string(),
            instruction: "Always use Rust".to_string(),
            priority: 1,
            created_at: 1000,
            updated_at: 1000,
        };
        let json = serde_json::to_string(&instr).unwrap();
        assert!(json.contains("Always use Rust"));
    }
}
