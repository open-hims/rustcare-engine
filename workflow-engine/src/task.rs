// Task definition and types stub
pub struct Task {
    pub name: String,
    pub task_type: TaskType,
}

impl Task {
    pub fn new(name: &str, task_type: TaskType) -> Self {
        Self {
            name: name.to_string(),
            task_type,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskType {
    HttpRequest,
    DatabaseOperation,
    Custom,
}