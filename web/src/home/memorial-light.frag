precision mediump float;
uniform vec2 resolution;
uniform float time;
void main() {
  vec2 uv = gl_FragCoord.xy / resolution;
  float light = 0.0;
  // Broad sunbeams remain in the decorative margin, away from the title.
  float beam = pow(max(0.0, sin(uv.x * 12.0 + uv.y * 1.8 + sin(time * 0.08) * 0.35)), 14.0);
  light += beam * 0.08 * smoothstep(0.35, 0.95, uv.x);
  for (int i = 0; i < 9; i++) {
    float n = float(i);
    vec2 point = vec2(0.5 + fract(n * 0.618) * 0.48, fract(n * 0.37 + time * 0.012));
    vec2 delta = (uv - point) * vec2(resolution.x / resolution.y, 1.0);
    light += exp(-dot(delta, delta) * 4200.0) * (0.16 + 0.06 * sin(time * 0.4 + n));
  }
  float alpha = min(light, 0.26);
  gl_FragColor = vec4(vec3(1.0, 0.79, 0.40) * alpha, alpha);
}
