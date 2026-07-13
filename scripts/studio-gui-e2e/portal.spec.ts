import { test, expect } from '@playwright/test';

test.describe('Qualia settings portal GUI', () => {
  test('health endpoint responds', async ({ request }) => {
    const res = await request.get('/health');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.status).toBe('ok');
  });

  test('design-studio spatial shell loads in browser', async ({ page }) => {
    const res = await page.goto('/design-studio.html');
    expect(res?.ok()).toBeTruthy();
    await expect(page).toHaveTitle(/design studio|qualia|portal/i);
  });

  test('generate_pane returns a layout plan', async ({ request }) => {
    const res = await request.post('/generate_pane', {
      data: {
        prompt: 'Health tracker with vitals and SPARQL',
        palette_ids: ['health-monitor', 'sparql-explorer'],
        use_llm: false,
      },
    });
    expect(res.ok()).toBeTruthy();
    const plan = await res.json();
    expect(plan.panes?.length).toBeGreaterThan(0);
  });

  test('undo-chain and WAL workflow', async ({ request }) => {
    const manifestV1 = {
      pages: [
        {
          url_path: '/',
          name: 'Home',
          panes: [{ component_id: 'n3-logic-studio', x: 0, y: 0, w: 40, h: 30, data_bindings: [] }],
          presentation_mode: 'GridBound',
        },
      ],
      theme_tokens: {},
      themes: [],
      environment_theme: {},
      app_theme: {},
    };

    const postUndo = await request.post('/manifest/undo-frame?stack_index=0', {
      data: manifestV1,
    });
    expect(postUndo.ok()).toBeTruthy();

    const chain = await request.get('/manifest/undo-chain');
    expect(chain.ok()).toBeTruthy();
    const body = await chain.json();
    expect(body.manifests?.length).toBeGreaterThan(0);
  });
});