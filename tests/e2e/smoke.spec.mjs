import { test, expect } from "@playwright/test";

test("browser smoke probe loads the app shell", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("body")).toBeVisible();
});
