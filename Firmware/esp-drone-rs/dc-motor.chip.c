// Minimal 8520 coreless brushed motor for ESP-Drone POC (Wokwi).
// V+ / V- accept logic-level motor terminal voltages (low-side MOSFET drive).
// ENC_A / ENC_B output quadrature; display shows prop rotation.

#include "wokwi-api.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TICK_US 100
#define DUTY_WINDOW_TICKS 100
#define PROP_COLOR 0x33CC33FF
#define HUB_COLOR 0x222222FF
#define BG_COLOR 0xF8F8F8FF

typedef struct {
  pin_t v_plus;
  pin_t v_minus;
  pin_t enc_a;
  pin_t enc_b;
  pin_t vel;
  timer_t timer;
  buffer_t fb;
  uint32_t fb_w;
  uint32_t fb_h;
  uint32_t supply_attr;
  uint32_t on_samples;
  uint32_t window_ticks;
  float duty;
  float velocity;
  float angle;
  int enc_phase;
} motor_state_t;

static void set_pixel(motor_state_t *m, int x, int y, uint32_t color) {
  if (x < 0 || y < 0 || (uint32_t)x >= m->fb_w || (uint32_t)y >= m->fb_h) {
    return;
  }
  uint32_t offset = (uint32_t)(y * m->fb_w + x) * 4;
  buffer_write(m->fb, offset, &color, 4);
}

static void fill_bg(motor_state_t *m) {
  uint32_t count = m->fb_w * m->fb_h;
  uint32_t *row = malloc(count * 4);
  if (!row) {
    return;
  }
  for (uint32_t i = 0; i < count; i++) {
    row[i] = BG_COLOR;
  }
  buffer_write(m->fb, 0, row, count * 4);
  free(row);
}

static void draw_prop(motor_state_t *m) {
  const int cx = (int)m->fb_w / 2;
  const int cy = (int)m->fb_h / 2;
  const int hub = 4;
  const int arm = 18;

  fill_bg(m);

  for (int dy = -hub; dy <= hub; dy++) {
    for (int dx = -hub; dx <= hub; dx++) {
      if (dx * dx + dy * dy <= hub * hub) {
        set_pixel(m, cx + dx, cy + dy, HUB_COLOR);
      }
    }
  }

  for (int blade = 0; blade < 3; blade++) {
    float a = m->angle + blade * (2.0f * (float)M_PI / 3.0f);
    float ca = cosf(a);
    float sa = sinf(a);
    for (int r = hub + 1; r <= arm; r++) {
      int x = cx + (int)(ca * r);
      int y = cy + (int)(sa * r);
      set_pixel(m, x, y, PROP_COLOR);
      set_pixel(m, x + 1, y, PROP_COLOR);
      set_pixel(m, x, y + 1, PROP_COLOR);
    }
  }
}

static void update_encoder(motor_state_t *m) {
  static const uint8_t phases[4][2] = {
      {0, 0},
      {1, 0},
      {1, 1},
      {0, 1},
  };
  int dir = m->velocity >= 0 ? 1 : -1;
  if (fabsf(m->velocity) < 0.02f) {
    return;
  }
  m->enc_phase = (m->enc_phase + dir + 4) % 4;
  pin_write(m->enc_a, phases[m->enc_phase][0]);
  pin_write(m->enc_b, phases[m->enc_phase][1]);
}

static void timer_tick(void *user_data) {
  motor_state_t *m = (motor_state_t *)user_data;

  uint32_t vp = pin_read(m->v_plus);
  uint32_t vm = pin_read(m->v_minus);
  if (vp > vm) {
    m->on_samples++;
  }
  m->window_ticks++;

  if (m->window_ticks >= DUTY_WINDOW_TICKS) {
    m->duty = m->window_ticks ? (float)m->on_samples / (float)m->window_ticks : 0.0f;
    m->on_samples = 0;
    m->window_ticks = 0;

    float supply = attr_read_float(m->supply_attr);
    if (supply < 0.1f) {
      supply = 3.3f;
    }
    float target = m->duty * supply / 3.3f;
    m->velocity += (target - m->velocity) * 0.25f;
    pin_dac_write(m->vel, m->velocity * supply);
  }

  if (fabsf(m->velocity) > 0.01f) {
    m->angle += m->velocity * 0.35f;
    if (m->angle > 2.0f * (float)M_PI) {
      m->angle -= 2.0f * (float)M_PI;
    }
    if (m->angle < 0.0f) {
      m->angle += 2.0f * (float)M_PI;
    }
    update_encoder(m);
    draw_prop(m);
  } else if (m->duty < 0.01f && fabsf(m->velocity) <= 0.01f) {
    draw_prop(m);
  }
}

void chip_init(void) {
  motor_state_t *m = calloc(1, sizeof(motor_state_t));
  m->v_plus = pin_init("V+", INPUT);
  m->v_minus = pin_init("V-", INPUT);
  m->enc_a = pin_init("ENC_A", OUTPUT);
  m->enc_b = pin_init("ENC_B", OUTPUT);
  m->vel = pin_init("vel", ANALOG);
  m->supply_attr = attr_init_float("supplyVolts", 3.3f);

  m->fb = framebuffer_init(&m->fb_w, &m->fb_h);
  draw_prop(m);

  timer_config_t cfg = {
      .user_data = m,
      .callback = timer_tick,
  };
  m->timer = timer_init(&cfg);
  timer_start(m->timer, TICK_US, true);
}
