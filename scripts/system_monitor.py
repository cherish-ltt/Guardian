#!/usr/bin/env python3
import asyncio
import signal
from datetime import datetime

import asyncpg
import croniter
import psutil

CONFIG = {
    "host": os.getenv("DB_HOST", "127.0.0.1"),
    "port": int(os.getenv("DB_PORT", 5432)),
    "user": os.getenv("DB_USER", "postgres"),
    "password": os.getenv("DB_PASSWORD"),
    "database": os.getenv("DB_NAME", "guardian_auth"),
}

DATABASE_URL = f"postgresql://{CONFIG['user']}:{CONFIG['password']}@{CONFIG['host']}:{CONFIG['port']}/{CONFIG['database']}"


class SystemMonitor:
    def __init__(self, db_url: str, cron_expression: str = "*/5 * * * *"):
        self.db_url = db_url
        self.pool = None
        self.last_network_stats = None
        self.running = True
        self.cron_expression = cron_expression

    async def init(self):
        self.pool = await asyncpg.create_pool(
            self.db_url, min_size=1, max_size=5, command_timeout=60
        )

    def get_next_run_time(self, current_time: datetime) -> datetime:
        """使用croniter获取下一次运行时间"""
        cron = croniter.croniter(self.cron_expression, current_time)
        return cron.get_next(datetime)

    def calculate_sleep_time(self, current_time: datetime) -> float:
        """计算到下一次运行需要等待的秒数"""
        next_run = self.get_next_run_time(current_time)

        # 计算时间差
        wait_seconds = (next_run - current_time).total_seconds()

        # 避免负数或过小的等待时间
        if wait_seconds < 1:
            wait_seconds = 1

        return wait_seconds

    async def collect_metrics(self):
        cpu_count = psutil.cpu_count(logical=False) or 1
        cpu_total_load = psutil.cpu_percent(interval=1)

        memory = psutil.virtual_memory()
        memory_used = memory.used
        memory_total = memory.total

        disk = psutil.disk_usage("/")
        disk_used = disk.used
        disk_total = disk.total

        network_stats = psutil.net_io_counters()
        if self.last_network_stats:
            network_upload = (
                network_stats.bytes_sent - self.last_network_stats.bytes_sent
            )
            network_download = (
                network_stats.bytes_recv - self.last_network_stats.bytes_recv
            )
        else:
            network_upload = 0
            network_download = 0
        self.last_network_stats = network_stats

        return {
            "cpu_count": cpu_count,
            "cpu_total_load": cpu_total_load,
            "memory_used": memory_used,
            "memory_total": memory_total,
            "disk_used": disk_used,
            "disk_total": disk_total,
            "network_upload": network_upload,
            "network_download": network_download,
        }

    async def insert_metrics(self, metrics: dict):
        async with self.pool.acquire() as conn:
            await conn.execute(
                """
                INSERT INTO guardian_systeminfo
                (cpu_count, cpu_total_load, memory_used, memory_total,
                 disk_used, disk_total, network_upload, network_download)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            """,
                metrics["cpu_count"],
                metrics["cpu_total_load"],
                metrics["memory_used"],
                metrics["memory_total"],
                metrics["disk_used"],
                metrics["disk_total"],
                metrics["network_upload"],
                metrics["network_download"],
            )

    async def run(self):
        await self.init()

        print(f"Starting system monitoring (cron expression: {self.cron_expression})")
        print("Collecting initial network stats...")

        # 信号处理，优雅关闭
        def signal_handler(signum, frame):
            print(f"\nReceived signal {signum}, shutting down...")
            self.running = False

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        # 先收集一次初始网络统计
        _ = await self.collect_metrics()
        print("Initial metrics collected, starting monitoring loop...")

        while self.running:
            try:
                # 获取当前时间
                now = datetime.now()

                # 计算下一次运行时间
                next_run_time = self.get_next_run_time(now)
                wait_seconds = self.calculate_sleep_time(now)

                print(f"Current time: {now.strftime('%Y-%m-%d %H:%M:%S')}")
                print(
                    f"Next run at: {next_run_time.strftime('%Y-%m-%d %H:%M:%S')} "
                    f"(in {wait_seconds:.1f} seconds)"
                )

                # 等待到下一次运行时间
                await asyncio.sleep(wait_seconds)

                # 检查是否在等待期间收到了关闭信号
                if not self.running:
                    break

                # 收集和存储指标
                metrics = await self.collect_metrics()
                await self.insert_metrics(metrics)

                timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                print(
                    f"[{timestamp}] CPU: {metrics['cpu_total_load']:.1f}%, "
                    f"Memory: {metrics['memory_used'] / 1024 / 1024 / 1024:.2f}GB/{metrics['memory_total'] / 1024 / 1024 / 1024:.2f}GB, "
                    f"Disk: {metrics['disk_used'] / 1024 / 1024 / 1024:.2f}GB/{metrics['disk_total'] / 1024 / 1024 / 1024:.2f}GB, "
                    f"Network: ↑{metrics['network_upload'] / 1024 / 1024:.2f}MB ↓{metrics['network_download'] / 1024 / 1024:.2f}MB"
                )

            except asyncio.CancelledError:
                print("Task cancelled, shutting down...")
                self.running = False
                break
            except Exception as e:
                print(f"Error during monitoring: {e}")
                # 出错时等待1分钟后重试
                await asyncio.sleep(60)

    async def close(self):
        if self.pool:
            await self.pool.close()


async def main():
    # 使用cron表达式，每5分钟运行一次
    cron_expression = "*/5 * * * *"  # 分钟 小时 日 月 星期

    monitor = SystemMonitor(DATABASE_URL, cron_expression)
    try:
        await monitor.run()
    except KeyboardInterrupt:
        print("\nShutting down monitor...")
    finally:
        await monitor.close()


if __name__ == "__main__":
    asyncio.run(main())
