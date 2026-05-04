// See https://svelte.dev/docs/kit/types
declare global {
  const __APP_VERSION__: string;
  const __APP_BUILD_ID__: string;
  const __APP_BUILD_TIME__: string;

  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface Platform {}
  }
}

export {};
