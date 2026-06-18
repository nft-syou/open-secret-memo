import { expect, test } from "@playwright/test";

test("encrypt then decrypt round-trips in a real browser", async ({ page }) => {
  await page.goto("/");

  // Encrypt
  await page.getByLabel("メモ本文", { exact: true }).fill("playwright secret");
  await page.getByLabel("合言葉", { exact: true }).fill("pw-e2e");
  await page.getByLabel("合言葉（確認）").fill("pw-e2e");
  await page.getByRole("button", { name: "暗号化する" }).click();

  const ciphertextField = page.getByLabel("暗号化済みテキスト");
  await expect(ciphertextField).toBeVisible();
  const ciphertext = await ciphertextField.inputValue();
  expect(ciphertext).toMatch(/^OSM1\./);

  // Decrypt — switch tab, then work inside the decrypt panel to avoid
  // ambiguity with the now-hidden encrypt panel's 暗号化済みテキスト field.
  await page.getByRole("tab", { name: "復号" }).click();
  const decryptPanel = page.locator("#panel-decrypt");
  await decryptPanel.getByLabel("暗号化済みテキスト").fill(ciphertext);
  await decryptPanel.getByLabel("合言葉", { exact: true }).fill("pw-e2e");
  await page.getByRole("button", { name: "復号する" }).click();

  await expect(decryptPanel.getByText("playwright secret")).toBeVisible();
});

test("still works offline after first load (PWA precache)", async ({ page, context }) => {
  await page.goto("/");
  // Wait until the service worker actually controls the page (precache done),
  // rather than a fixed sleep — robust across slow/fast CI machines. The PWA is
  // generated with clientsClaim, so `controller` is set once the SW activates.
  await page.waitForFunction(() => !!navigator.serviceWorker?.controller, undefined, {
    timeout: 20_000,
  });
  await context.setOffline(true);
  await page.reload();

  await page.getByLabel("メモ本文", { exact: true }).fill("offline secret");
  await page.getByLabel("合言葉", { exact: true }).fill("pw-off");
  await page.getByLabel("合言葉（確認）").fill("pw-off");
  await page.getByRole("button", { name: "暗号化する" }).click();
  await expect(page.getByLabel("暗号化済みテキスト")).toHaveValue(/^OSM1\./);
});
