"""日志时间筛选浏览器回归；仅连接独立测试 SQLite 库，夹具按唯一关键词自建自删。

GUARDIAN_TEST_URL / GUARDIAN_TEST_DB / GUARDIAN_TEST_PASSWORD 指定联调环境。
使用已安装的 Playwright 和 Chromium；可用 GUARDIAN_TEST_BROWSER 指定浏览器路径。
"""
import asyncio
import csv
import io
import os
from pathlib import Path
import secrets
import sqlite3
from urllib.parse import parse_qs, urlparse

from playwright.async_api import async_playwright, expect

from captcha_solver import solve_captcha


async def main():
    base = os.environ.get("GUARDIAN_TEST_URL", "http://127.0.0.1:5173")
    database = Path(os.environ["GUARDIAN_TEST_DB"]).resolve(strict=True)
    password = os.environ["GUARDIAN_TEST_PASSWORD"]
    keyword = "time-window-" + secrets.token_hex(6)
    start, end = "2026-01-02 10:00:00", "2026-01-02 11:00:00"
    logs = [
        ("oper-logs", "oper-log", "sys_oper_log", "module", "oper_time", "result", "action"),
        ("login-logs", "login-log", "sys_login_log", "username", "login_time", "status", "msg"),
    ]
    with sqlite3.connect(database) as db:
        for _, _, table, name, time, status, detail in logs:
            rows = [(keyword, start, 0, "fixture")] + [(keyword, "2026-01-02 10:30:00", 0, "fixture")] * 11
            rows += [(keyword, t, s, "fixture") for t, s in [("2026-01-02 09:59:59", 0), (end, 0), ("2026-01-02 11:00:01", 0), (start, 1)]]
            db.executemany(f"INSERT INTO {table} ({name},{time},{status},{detail}) VALUES (?,?,?,?)", rows)
    try:
        async with async_playwright() as pw:
            browser = await pw.chromium.launch(executable_path=os.environ.get("GUARDIAN_TEST_BROWSER"))
            try:
                page = await browser.new_page(viewport={"width": 1440, "height": 1000}, timezone_id="America/New_York")
                errors = []
                page.on("pageerror", lambda error: errors.append(str(error)))
                await page.goto(base + "/login")
                await page.get_by_placeholder("请输入账号", exact=True).fill("admin")
                await page.get_by_placeholder("请输入密码", exact=True).fill(password)
                image = page.locator(".captcha-image")
                await expect(image).to_be_visible()
                await page.get_by_placeholder("请输入右侧字符", exact=True).fill(solve_captcha(await image.get_attribute("src")))
                await page.get_by_role("button", name="登录", exact=True).click()
                await page.wait_for_url(lambda url: "/login" not in urlparse(url).path)

                for resource, route, _, _, _, status_field, _ in logs:
                    await page.goto(base + "/monitor/" + route)
                    form = page.get_by_role("search")
                    await expect(form).to_be_visible()
                    await form.locator(".field").filter(has_text="关键词").locator("input").fill(keyword)
                    await form.get_by_role("combobox").click()
                    await page.get_by_role("option", name="失败", exact=True).click()
                    start_input = form.get_by_label("开始时间", exact=False)
                    end_input = form.get_by_label("结束时间", exact=False)
                    await start_input.fill(start)
                    await end_input.fill(end)

                    async def query(action, expected_total, expected_start=start, expected_end=end, expected_page="1", expected_keyword=keyword):
                        async with page.expect_response(lambda response: urlparse(response.url).path == f"/api/v1/monitor/{resource}") as pending:
                            await action()
                        response = await pending.value
                        assert response.status == 200, await response.text()
                        params = parse_qs(urlparse(response.url).query)
                        assert params.get("startTime") == ([expected_start] if expected_start else None), params
                        assert params.get("endTime") == ([expected_end] if expected_end else None), params
                        assert params.get("pageNum") == [expected_page], params
                        assert params.get("keyword") == ([expected_keyword] if expected_keyword else None), params
                        if expected_keyword:
                            assert params.get(status_field) == ["0"], params
                        data = (await response.json())["data"]
                        if expected_total is not None:
                            assert data["total"] == expected_total, data
                        return data

                    search = form.get_by_role("button", name="查询", exact=True).click
                    data = await query(search, 12)
                    assert len(data["rows"]) == 10
                    data = await query(page.get_by_role("button", name="下一页", exact=True).click, 12, expected_page="2")
                    assert len(data["rows"]) == 2
                    async with page.expect_download() as downloaded, page.expect_response(lambda response: urlparse(response.url).path == f"/api/v1/monitor/{resource}/export") as exported:
                        await page.get_by_role("button", name="导出当前查询结果", exact=True).click()
                    response = await exported.value
                    assert response.status == 200
                    params = parse_qs(urlparse(response.url).query)
                    assert params == {"keyword": [keyword], status_field: ["0"], "startTime": [start], "endTime": [end]}, params
                    download = await downloaded.value
                    rows = list(csv.reader(io.StringIO(Path(await download.path()).read_text())))
                    assert len(rows) == 13, rows
                    await download.delete()
                    await query(form.get_by_role("button", name="重置", exact=True).click, None, None, None, expected_keyword=None)
                    await expect(start_input).to_have_value("")
                    await expect(end_input).to_have_value("")
                    await form.locator(".field").filter(has_text="关键词").locator("input").fill(keyword)
                    await form.get_by_role("combobox").click()
                    await page.get_by_role("option", name="失败", exact=True).click()
                    await start_input.fill(start)
                    await end_input.fill(end)
                    await form.locator(".field").filter(has_text="结束时间").get_by_role("button", name="清空", exact=True).click()
                    await query(search, 14, expected_end=None)
                    await start_input.fill("2027-01-01 00:00:00")
                    await query(search, 0, "2027-01-01 00:00:00", None)
                    await expect(page.locator(".empty")).to_be_visible()
                    await start_input.fill("invalid")
                    async with page.expect_response(lambda response: urlparse(response.url).path == f"/api/v1/monitor/{resource}") as invalid:
                        await search()
                    assert (await invalid.value).status == 400
                    await expect(page.locator(".toast").filter(has_text="startTime")).to_be_visible()
                    await page.locator(".toast").filter(has_text="startTime").get_by_role("button").click()
                    await start_input.fill(start)
                    await end_input.fill(end)
                    await query(search, 12)
                    for width in [1440, 390]:
                        await page.set_viewport_size({"width": width, "height": 1000})
                        await form.scroll_into_view_if_needed()
                        await expect(start_input).to_be_visible()
                        await query(search, 12)
                        if output := os.environ.get("GUARDIAN_TEST_SCREENSHOTS"):
                            Path(output).mkdir(parents=True, exist_ok=True)
                            await page.screenshot(path=str(Path(output) / f"{resource}-{width}.png"))
                        assert await start_input.evaluate("el => el.getBoundingClientRect().right <= innerWidth"), await start_input.bounding_box()
                    await page.set_viewport_size({"width": 1440, "height": 1000})
                    print(f"PASS {resource}: boundaries, pagination, CSV, reset, clear, empty, 400, desktop/mobile, no timezone conversion")
                assert not errors, errors
            finally:
                await browser.close()
    finally:
        with sqlite3.connect(database) as db:
            for _, _, table, name, _, _, _ in logs:
                db.execute(f"DELETE FROM {table} WHERE {name} = ?", (keyword,))


if __name__ == "__main__":
    asyncio.run(main())
