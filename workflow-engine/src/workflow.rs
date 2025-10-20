// Workflow definition stub
pub struct Workflow {
    pub name: String,
}

impl Workflow {
    pub fn builder(name: &str) -> WorkflowBuilder {
        WorkflowBuilder::new(name)
    }
}

pub struct WorkflowBuilder {
    name: String,
}

impl WorkflowBuilder {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
    
    pub fn add_task(self, _task: crate::task::Task) -> Self {
        self // Stub implementation
    }
    
    pub fn build(self) -> Workflow {
        Workflow { name: self.name }
    }
}