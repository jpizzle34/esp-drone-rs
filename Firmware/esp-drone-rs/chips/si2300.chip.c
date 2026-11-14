// Si2300DS N-channel MOSFET, low-side switch for Wokwi.
// GPIO drives the gate directly. Gate HIGH pulls the drain to the source
// (GND); gate LOW releases the drain so the motor winding pulls it high.
// Digital stand-in: Wokwi pins are HIGH/LOW, so this does not model RDS(on).

#include "wokwi-api.h"
#include <stdio.h>
#include <stdlib.h>

typedef struct {
  pin_t gate;
  pin_t drain;
  pin_t source;
} chip_state_t;

static void gate_changed(void *user_data, pin_t pin, uint32_t level) {
  chip_state_t *m = (chip_state_t *)user_data;
  (void)pin;

  if (level) {
    pin_write(m->drain, pin_read(m->source));
  } else {
    pin_write(m->drain, HIGH);
  }
}

void chip_init(void) {
  chip_state_t *m = malloc(sizeof(chip_state_t));
  m->gate = pin_init("GATE", INPUT);
  m->drain = pin_init("DRAIN", OUTPUT);
  m->source = pin_init("SOURCE", INPUT);

  if (pin_read(m->gate)) {
    pin_write(m->drain, pin_read(m->source));
  } else {
    pin_write(m->drain, HIGH);
  }

  const pin_watch_config_t cfg = {
      .edge = BOTH,
      .pin_change = gate_changed,
      .user_data = m,
  };
  pin_watch(m->gate, &cfg);
}
