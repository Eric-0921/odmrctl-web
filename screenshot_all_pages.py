#!/usr/bin/env python3
"""Screenshot all GUI-M0 pages at 1920x1200 for 4K external monitor."""

from playwright.sync_api import sync_playwright
import os

BASE = "http://localhost:8765"
OUT_DIR = "/Users/erictseng/Documents/codex_git/odmrctl-web/screenshots"

# All routes with their display names and file suffixes
ROUTES = [
    ("", "Dashboard", "dashboard"),
    ("#/devices", "Devices", "devices"),
    ("#/recipe", "Recipe", "recipe"),
    ("#/dry-run", "Dry Run", "dry-run"),
    ("#/safety", "Safety", "safety"),
    ("#/events", "Events", "events"),
    ("#/raw-data", "Raw Data Preview", "raw-data"),
    ("#/analysis-viewer", "Analysis Viewer", "analysis-viewer"),
    ("#/recipe-viewer", "Recipe Viewer", "recipe-viewer"),
    ("#/about", "About / Boundaries", "about"),
]

# System Scan tabs
SCAN_TABS = [
    ("Overview", "system-scan-overview"),
    ("Recipe", "system-scan-recipe"),
    ("Station Safety", "system-scan-station-safety"),
    ("Device Profiles", "system-scan-device-profiles"),
    ("Resolved Steps", "system-scan-resolved-steps"),
    ("Safety Report", "system-scan-safety-report"),
    ("Dry Run", "system-scan-dry-run"),
]


def screenshot_route(page, path, name, suffix):
    url = f"{BASE}/{path}"
    page.goto(url, wait_until="networkidle")
    page.wait_for_timeout(800)
    out = os.path.join(OUT_DIR, f"{suffix}.png")
    page.screenshot(path=out, full_page=False)
    print(f"  {out}")


def screenshot_system_scan(page):
    url = f"{BASE}/#/system-scan"
    page.goto(url, wait_until="networkidle")
    page.wait_for_timeout(800)

    for label, suffix in SCAN_TABS:
        btn = page.locator("button", has_text=label).first
        if btn.is_visible():
            btn.click()
            page.wait_for_timeout(600)
        else:
            page.get_by_text(label, exact=False).first.click()
            page.wait_for_timeout(600)

        out = os.path.join(OUT_DIR, f"{suffix}.png")
        page.screenshot(path=out, full_page=False)
        print(f"  {out}")


def main():
    os.makedirs(OUT_DIR, exist_ok=True)

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1920, "height": 1200})

        for path, name, suffix in ROUTES:
            screenshot_route(page, path, name, suffix)

        screenshot_system_scan(page)

        browser.close()
    print("Done.")


if __name__ == "__main__":
    main()
