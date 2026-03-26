//! Monitoring and watchdog module.
//! 负责采集与汇总运行时状态，用于实现类似 systemd/watchdog 的健康监控机制：
//! - heartbeat：记录每台 Robot 的心跳时间戳
//! - metrics：记录每台 Robot / 全局的执行统计
//! - health_checker：基于心跳和指标给出健康评估
//! - reporter：面向 API / 前端生成易消费的报告结构
//! - monitor_thread：后台监控线程，定期评估健康状态并打印日志

pub mod health_checker;
pub mod heartbeat;
pub mod metrics;
pub mod monitor_thread;
pub mod reporter;
