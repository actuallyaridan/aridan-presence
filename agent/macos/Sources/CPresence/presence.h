// The Rust side's functions (src/lib.rs), as Swift sees them. Every char *
// they give back is Rust's: copy it, then hand it to presence_free_string.

#pragma once

#include <stdbool.h>

// Starts the engine. on_status is called, from another thread, with every
// new status as JSON.
void presence_start(void (*on_status)(char *json));

char *presence_status_json(void);
char *presence_config_json(void);
char *presence_log_json(void);
char *presence_config_path(void);
char *presence_version(void);

// An empty string if it worked, or why not.
char *presence_save_config(const char *json);

void presence_set_paused(bool paused);

// Takes this computer off the site, then calls done, from another thread.
void presence_quit(void (*done)(void));

void presence_free_string(char *text);
