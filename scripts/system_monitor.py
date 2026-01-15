#!/usr/bin/env python3
import asyncio
import asyncpg
import psutil
import os
from datetime import datetime, timezone

CONFIG = {
    "host": "127.0.0.1",
    "port": 5432,
    "user": "postgres",
    "password": "123456",
    "database": "guardian_auth",
}

DATABASE_URL = f"postgresql://{CONFIG['user']}:{CONFIG['password']}@{CONFIG['host']}:{CONFIG['port']}/{CONFIG['database']}"


class SystemMonitor:
    def __init__(self, db_url: str):
        self.db_url = db_url
        self.pool = None
        self.last_network_stats = None

    async def init(self):
        self.pool = await asyncpg.create_pool(
            self.db_url, min_size=1, max_size=5, command_timeout=60
        )

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

    async def run(self, interval: int = 300):
        await self.init()

        print(f"Starting system monitoring (interval: {interval}s)")
        print("Collecting initial network stats...")

        while True:
            metrics = await self.collect_metrics()
            await self.insert_metrics(metrics)

            timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
            print(
                f"[{timestamp}] CPU: {metrics['cpu_total_load']:.1f}%, "
                f"Memory: {metrics['memory_used'] / 1024 / 1024 / 1024:.2f}GB/{metrics['memory_total'] / 1024 / 1024 / 1024:.2f}GB, "
                f"Disk: {metrics['disk_used'] / 1024 / 1024 / 1024:.2f}GB/{metrics['disk_total'] / 1024 / 1024 / 1024:.2f}GB"
            )

            await asyncio.sleep(interval)

    async def close(self):
        if self.pool:
            await self.pool.close()


async def main():
    monitor = SystemMonitor(DATABASE_URL)
    try:
        await monitor.run(interval=300)
    except KeyboardInterrupt:
        print("\nShutting down monitor...")
        await monitor.close()


if __name__ == "__main__":
    asyncio.run(main())
