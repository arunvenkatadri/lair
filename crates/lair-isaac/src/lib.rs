//! LAIR Isaac — simulation bridge interface.
//!
//! Defines the `SimBridge` trait for connecting LAIR task graphs to
//! simulation environments (NVIDIA Isaac Sim, Gazebo, etc.).

use lair_core::prelude::LairResult;

/// Bridge between LAIR and an external simulator.
///
/// Implementations handle the transport layer (shared memory, gRPC, etc.)
/// to exchange sensor data and control commands with the sim.
pub trait SimBridge {
    /// Advance the simulation by `dt_ns` nanoseconds.
    fn step(&mut self, dt_ns: u64) -> LairResult<()>;

    /// Send a command to the simulator on the given channel.
    fn send_command(&mut self, channel: &str, data: &[u8]) -> LairResult<()>;

    /// Receive sensor data from the simulator on the given channel.
    fn receive_sensor(&mut self, channel: &str) -> LairResult<Vec<u8>>;

    /// Reset the simulation to its initial state.
    fn reset(&mut self) -> LairResult<()>;

    /// Check whether the simulator is connected and responsive.
    fn is_connected(&self) -> bool;
}

/// In-memory mock simulator for testing.
///
/// Does not require NVIDIA Isaac Sim or any external process.
/// Stores the last command sent and returns empty sensor data.
pub struct MockSimBridge {
    connected: bool,
    last_channel: String,
    last_command: Vec<u8>,
    time_ns: u64,
}

impl MockSimBridge {
    pub fn new() -> Self {
        Self {
            connected: true,
            last_channel: String::new(),
            last_command: Vec::new(),
            time_ns: 0,
        }
    }

    /// Returns the last channel a command was sent to.
    pub fn last_channel(&self) -> &str {
        &self.last_channel
    }

    /// Returns the last command data sent.
    pub fn last_command(&self) -> &[u8] {
        &self.last_command
    }

    /// Returns the accumulated simulation time in nanoseconds.
    pub fn time_ns(&self) -> u64 {
        self.time_ns
    }
}

impl Default for MockSimBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SimBridge for MockSimBridge {
    fn step(&mut self, dt_ns: u64) -> LairResult<()> {
        self.time_ns += dt_ns;
        Ok(())
    }

    fn send_command(&mut self, channel: &str, data: &[u8]) -> LairResult<()> {
        self.last_channel = channel.to_string();
        self.last_command = data.to_vec();
        Ok(())
    }

    fn receive_sensor(&mut self, _channel: &str) -> LairResult<Vec<u8>> {
        Ok(Vec::new())
    }

    fn reset(&mut self) -> LairResult<()> {
        self.time_ns = 0;
        self.last_channel.clear();
        self.last_command.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_bridge_step_advances_time() {
        let mut bridge = MockSimBridge::new();
        assert_eq!(bridge.time_ns(), 0);
        bridge.step(1_000_000).unwrap(); // 1ms
        assert_eq!(bridge.time_ns(), 1_000_000);
        bridge.step(500_000).unwrap();
        assert_eq!(bridge.time_ns(), 1_500_000);
    }

    #[test]
    fn mock_bridge_send_and_receive() {
        let mut bridge = MockSimBridge::new();
        bridge.send_command("motor", &[1, 2, 3]).unwrap();
        assert_eq!(bridge.last_channel(), "motor");
        assert_eq!(bridge.last_command(), &[1, 2, 3]);

        let sensor_data = bridge.receive_sensor("lidar").unwrap();
        assert!(sensor_data.is_empty());
    }

    #[test]
    fn mock_bridge_reset() {
        let mut bridge = MockSimBridge::new();
        bridge.step(1_000).unwrap();
        bridge.send_command("ch", &[42]).unwrap();
        bridge.reset().unwrap();
        assert_eq!(bridge.time_ns(), 0);
        assert!(bridge.last_command().is_empty());
    }

    #[test]
    fn mock_bridge_is_connected() {
        let bridge = MockSimBridge::new();
        assert!(bridge.is_connected());
    }
}
