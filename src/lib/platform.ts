/** True inside the Android/iOS build: no window chrome, one column, touch-sized controls. */
export const isMobile = /android|iphone|ipad/i.test(navigator.userAgent);
