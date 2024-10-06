use crate::paths::o8::Instruction8;

// "Overseer"? Give it a more fun name
pub struct Optimizer {
    graph_put_instruction_channel: flume::Sender<Instruction8>,
}

impl Optimizer {
    pub fn new(graph_put_instruction_channel: flume::Sender<Instruction8>) -> Self {
        Self {
            graph_put_instruction_channel,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_put_instruction_channel
            .send(Instruction8::StartPort(8080))?;
        self.graph_put_instruction_channel
            .send(Instruction8::StartPort(8081))?;
        Ok(())
    }
}
