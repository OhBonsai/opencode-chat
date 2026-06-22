// PixelSpiritDeck 50 icon —— GLSL contact-sheet(10×5)。
// plan16 §2.5 整盘移植源(vendored-with-risk,见 §6)。供 gallery 内联渲染。
// 注:iMouse hover 预览已删(画廊不需要)。

#define S (1. + sin(iTime) / 4.)
#define C (1. + cos(iTime) / 4.)

float sstep(float a, float b) { return smoothstep(a - .005, a + .005, b); }

const float PI = 3.14159;
const float TAU = PI * 2.;
const float QTR_PI = PI / 4.;

float stroke(float x, float s, float w) {
    float d = sstep(s, x + w / 2.) - sstep(s, x - w / 2.);
    return clamp(d, 0., 1.);
}
float circleSDF(vec2 st) { return length(st - 0.5) * 2.; }
float fill(float x, float size) { return 1. - sstep(size, x); }
float rectSDF(vec2 st, vec2 s) {
    st = st * 2. - 1.;
    return max(abs(st.x / s.x), abs(st.y / s.y));
}
float crossSDF(vec2 st, float s) {
    vec2 size = vec2(0.25, s);
    return min(rectSDF(st, size.xy), rectSDF(st, size.yx));
}
float flip(float v, float pct) { return mix(v, 1. - v, pct); }
float vesicaSDF(vec2 st, float w) {
    vec2 offset = vec2(w * .5, 0.);
    return max(circleSDF(st - offset), circleSDF(st + offset));
}
float triSDF(vec2 st) {
    st = (2. * st - 1.) * 2.;
    return max(abs(st.x) * 0.866025 + st.y * .5, -st.y * .5);
}
float rhombSDF(vec2 st) { return max(triSDF(st), triSDF(vec2(st.x, 1. - st.y))); }
vec2 rotate(vec2 st, float a) {
    st = mat2(cos(a), -sin(a), sin(a), cos(a)) * (st - .5);
    return st + .5;
}
float polySDF(vec2 st, int V) {
    st = st * 2. - 1.;
    float a = atan(st.x, st.y) + PI;
    float r = length(st);
    float v = TAU / float(V);
    return cos(floor(.5 + a / v) * v - a) * r;
}
float hexSDF(vec2 st) {
    st = abs(st * 2. - 1.);
    return max(abs(st.y), st.x * 0.866025 + st.y * .5);
}
float starSDF(vec2 st, int V, float s) {
    st = st * 4. - 2.;
    float a = atan(st.y, st.x) / TAU;
    float seg = a * float(V);
    a = ((floor(seg) + 0.5) / float(V) + mix(s, -s, step(.5, fract(seg)))) * TAU;
    return abs(dot(vec2(cos(a), sin(a)), st));
}
float raysSDF(vec2 st, int N) {
    st -= .5;
    return fract(atan(st.y, st.x) / TAU * float(N));
}
float heartSDF(vec2 st) {
    st -= vec2(.5, .8);
    float r = length(st) * 5.;
    st = normalize(st);
    return r - ((st.y * pow(abs(st.x), 0.67)) / (st.y + 1.5) - 2. * st.y + 1.26);
}
float bridge(float c, float d, float s, float w) {
    c *= 1. - stroke(d, s, w * 2.);
    return c + stroke(d, s, w);
}
float spiralSDF(vec2 st, float t) {
    st -= .5;
    float r = dot(st, st);
    float a = atan(st.y, st.x);
    return abs(sin(fract(log(r) * t + a * 0.159)));
}
vec2 scale(vec2 st, vec2 s) { return (st - .5) * s + .5; }
float flowerSDF(vec2 st, int N) {
    st = st * 2. - 1.;
    float r = length(st) * 2.;
    float a = atan(st.y, st.x);
    float v = float(N) * .5;
    return 1. - (abs(cos(a * v)) * .5 + .5) / r;
}

float draw(vec2 st, vec2 tileXY, vec2 count) {
    int cardNumber = int(tileXY.x + (-tileXY.y + count.y - 1.) * count.x);
    float color = 0.;
    switch (cardNumber) {
    case 0: { color = 0.; break; }
    case 1: { color = sstep(0.5 * S, st.x); break; }
    case 2: { color = sstep(0.5 + cos(st.y * PI + iTime/2.) * 0.25, st.x); break; }
    case 3: { color = sstep(0.5, (st.x * S + st.y * C) * 0.5); break; }
    case 4: { color = stroke(st.x, 0.5, 0.15*S); break; }
    case 5: {
        float offset = cos(st.y * PI + iTime) * 0.15;
        color = stroke(st.x, .28 + offset, 0.1);
        color += stroke(st.x, .5 + offset, 0.1);
        color += stroke(st.x, .72 + offset, 0.1);
        break;
    }
    case 6: {
        float offset = 0.5 + (st.x - st.y) * 0.5;
        color = stroke(offset, 0.5, 0.1 * S);
        break;
    }
    case 7: {
        float sdf = 0.5 + (st.x - st.y) * 0.5;
        color = stroke(sdf, 0.5, 0.1 * C);
        float sdf_inv = (st.x + st.y) * 0.5;
        color += stroke(sdf_inv, 0.5, 0.1 * C);
        break;
    }
    case 8: { color = stroke(circleSDF(st), 0.5 * S, 0.05 * C); break; }
    case 9: {
        color = fill(circleSDF(st), 0.65);
        vec2 offset = vec2(0.1, 0.05);
        color -= fill(circleSDF(st - offset * S), 0.5);
        break;
    }
    case 10: {
        float sdf = rectSDF(st, vec2(1.));
        color = stroke(sdf, .5 * C, .125);
        color += fill(sdf, .1 * S);
        break;
    }
    case 11: {
        float rect = rectSDF(st, vec2(1));
        color = fill(rect, .5);
        float cross = crossSDF(st, 1.);
        color *= sstep(.5, fract(cross * 3. + iTime));
        color *= sstep(1., cross);
        color += fill(cross, .5);
        color += stroke(rect, .65, .05);
        color += stroke(rect, .75, .025);
        break;
    }
    case 12: {
        float rect = rectSDF(st, vec2(.5, 1.));
        float diag = (st.x * C + st.y * S) * .5;
        color = flip(fill(rect, .6), stroke(diag, .5, .01));
        break;
    }
    case 13: {
        vec2 offset = vec2(.15 * S, 0);
        float left = circleSDF(st + offset);
        float right = circleSDF(st - offset);
        color = flip(stroke(left, .5, .05), fill(right, 0.525));
        break;
    }
    case 14: {
        float sdf = vesicaSDF(st, .2 * S);
        color = flip(fill(sdf, .5), sstep((st.x + st.y) * .5, .5));
        break;
    }
    case 15: {
        st.y = 1. - st.y;
        vec2 ts = vec2(st.x, .82 - st.y);
        color = fill(triSDF(st), .7);
        color -= fill(triSDF(ts), .36);
        break;
    }
    case 16: {
        float circle = circleSDF(st - vec2(.0, .1));
        float triangle = triSDF(st + vec2(.0, .1));
        color = stroke(circle, .5 * C, .1);
        color *= sstep(.55, triangle);
        color += fill(triangle, .45);
        break;
    }
    case 17: {
        float sdf = rhombSDF(st);
        color = fill(sdf, .425 * S);
        color += stroke(sdf, .5 * S, .05);
        color += stroke(sdf, .6 * C, .03);
        break;
    }
    case 18: { color = flip(fill(triSDF(st), .5), fill(rhombSDF(st), .4)); break; }
    case 19: {
        st = rotate(st, radians(-25.) * S);
        float sdf = triSDF(st);
        sdf /= triSDF(st + vec2(0., .2 * C));
        color = fill(abs(sdf), .56);
        break;
    }
    case 20: {
        st = rotate(st, radians(45.));
        color = fill(rectSDF(st, vec2(1.)), .4);
        color *= 1. - stroke(st.x, .5 * S, .02);
        color *= 1. - stroke(st.y, .5 * C, .02);
        break;
    }
    case 21: {
        st = rotate(st, radians(-45.));
        float off = .12 * S;
        vec2 s = vec2(1.);
        color = fill(rectSDF(st + off, s), .2 * C);
        color += fill(rectSDF(st - off, s), .2 * C);
        float r = rectSDF(st, s);
        color *= sstep(.33, r);
        color += fill(r, .3);
        break;
    }
    case 22: {
        st = rotate(vec2(st.x, 1. - st.y), radians(45.));
        vec2 s = vec2(1.);
        color += fill(rectSDF(st - .025 * S, s), .4);
        color += fill(rectSDF(st + .025, s), .4);
        color *= sstep(0.38, rectSDF(st + .025, s));
        break;
    }
    case 23: {
        st = rotate(st, radians(-45.));
        vec2 s = vec2(1.);
        float o = .05 * S * 1.5;
        color += flip(fill(rectSDF(st - o, s), .4), fill(rectSDF(st + o, s), .4));
        break;
    }
    case 24: {
        st = rotate(st, radians(45.));
        float r1 = rectSDF(st, vec2(1.) * S);
        float r2 = rectSDF(st + .15 * S, vec2(1.));
        color += stroke(r1, .5, .05);
        color *= sstep(.325, r2);
        color += stroke(r2, .325, .05) * fill(r1, .525);
        color += stroke(r2, .2, .05);
        break;
    }
    case 25: {
        st = rotate(st, radians(-45.)) - .08;
        for (int i = 0; i < 4; i++) {
            float r = rectSDF(st, vec2(1.) * S);
            color += stroke(r, .19, .04);
            st += .05;
        }
        break;
    }
    case 26: {
        float d1 = polySDF(st, 5);
        vec2 ts = vec2(st.x, 1. - st.y);
        float d2 = polySDF(ts, 5);
        color = fill(d1, .75) * fill(fract(d1 * 5. - iTime/2.), .5);
        color -= fill(d1, .6) * fill(fract(d2 * 4.9 - iTime/2.), .45);
        break;
    }
    case 27: {
        st = st.yx;
        color = stroke(hexSDF(st), .6 * C, .1);
        color += fill(hexSDF(st - vec2(-.06, -.1) * S), .15);
        color += fill(hexSDF(st - vec2(-.06, .1) * S), .15);
        color += fill(hexSDF(st - vec2(.11, 0.) * S), .15);
        break;
    }
    case 28: {
        color += stroke(circleSDF(st), .8 * C, .05);
        st.y = 1. - st.y;
        float s = starSDF(st.yx, 5, .1);
        color *= sstep(.7 * C, s);
        color += stroke(s, .4 * S, .1);
        break;
    }
    case 29: {
        float bg = starSDF(st, 16, .1 * S);
        color += fill(bg, 1.3);
        float l = 0.;
        for (float i = 0.; i < 8.; i++) {
            vec2 xy = rotate(st, QTR_PI * i+iTime/4.);
            xy.y -= .3;
            float tri = polySDF(xy, 3);
            color += fill(tri, .3);
            l += stroke(tri, .3 * S, .03);
        }
        color *= 1. - l;
        float c = polySDF(st, 8);
        color -= stroke(c, .15, .04);
        break;
    }
    case 30: {
        color = stroke(raysSDF(st, 8), .5, .15 * C * 2.);
        float inner = starSDF(st.xy, 6, .09 * S);
        float outer = starSDF(st.yx, 6, .09 * S);
        color *= sstep(.7, outer);
        color += fill(outer, .5);
        color -= stroke(inner, .25, .06);
        color += stroke(outer, .6, .05);
        break;
    }
    case 31: {
        color = flip(stroke(raysSDF(rotate(st, -iTime/8.), 28), .5, .2), fill(st.y, .5));
        float rect = rectSDF(st, vec2(1) * S);
        color *= sstep(.25, rect);
        color += fill(rect, .2);
        break;
    }
    case 32: {
        float sdf = polySDF(rotate(st.yx, C), 8);
        color = fill(sdf, .5);
        color *= stroke(raysSDF(rotate(st, C), 8), .5, .2);
        color *= sstep(.27, sdf);
        color += stroke(sdf, .2, .05);
        color += stroke(sdf, .6, .1);
        break;
    }
    case 33: {
        float v1 = vesicaSDF(st, .5);
        vec2 st2 = st.yx + vec2(.04, .0);
        float v2 = vesicaSDF(st2, .7);
        color = stroke(v2, 1., .05);
        st = rotate(st, iTime/2.);
        color += fill(v2, 1.) * stroke(circleSDF(st - vec2(.05)), .3 , .05);
        color += fill(raysSDF(st, 50), .2) * fill(v1, 1.25) * sstep(1., v2);
        break;
    }
    case 34: {
        color = fill(heartSDF(st), .5 * C * 1.2);
        color -= stroke(polySDF(st, 3), .15 * S * 1.1, .05);
        break;
    }
    case 35: {
        st.x = flip(st.x, step(.5, st.y));
        vec2 offset = vec2(.15 * S, .0);
        float left = circleSDF(st + offset);
        float right = circleSDF(st - offset);
        color = stroke(left, .4 * S, .075);
        color = bridge(color, right, .4 * S, .075);
        break;
    }
    case 36: {
        st = st.yx;
        st.x = mix(1. - st.x, st.x, step(.5, st.y));
        vec2 o = vec2(.1, .0);
        vec2 s = vec2(1.) * C;
        float a = radians(45.) + iTime/2.;
        float l = rectSDF(rotate(st + o, a), s);
        float r = rectSDF(rotate(st - o, -a), s);
        color = stroke(l, .3, .1);
        color = bridge(color, r, .3, .1);
        color += fill(rhombSDF(abs(st.yx - vec2(.0, .5))), .1);
        break;
    }
    case 37: {
        st.x = mix(1. - st.x, st.x, step(.5, st.y));
        vec2 o = vec2(.05, .0);
        vec2 s = vec2(1.);
        float a = radians(45.);
        float l = rectSDF(rotate(st + o, a * S), s);
        float r = rectSDF(rotate(st - o, -a * S), s);
        color = stroke(l, .145, .098);
        color = bridge(color, r, .145, .098);
        break;
    }
    case 38: {
        float r1 = rectSDF(st, vec2(1.));
        float r2 = rectSDF(rotate(st, radians(45.)), vec2(1.));
        float inv = step(.5, (st.x + st.y) * .5);
        inv = flip(inv, step(.5, .5 + (st.x - st.y) * .5));
        float w = .075 * S * 1.2;
        color = stroke(r1, .5, w) + stroke(r2, .5, w);
        float bridges = mix(r1, r2, inv);
        color = bridge(color, bridges, .5, w);
        break;
    }
    case 39: {
        float inv = sstep(.5, st.y);
        st = rotate(st, radians(-45.)) - .2;
        st = mix(st, .6 - st, sstep(.5, inv));
        for (int i = 0; i < 5; i++) {
            float r = rectSDF(st, vec2(1.));
            float s = .25;
            s -= abs(float(i) * .1 - .2);
            color = bridge(color, r, s, .05 * S);
            st += .1;
        }
        break;
    }
    case 40: {
        st = rotate(st, radians(-60.) + iTime/4.);
        st.y = flip(st.y, step(.5, st.x));
        st.y += .25;
        float down = polySDF(st, 3);
        st.y = 1.5 - st.y;
        float top = polySDF(st, 3);
        color = stroke(top, .4, .15 * S);
        color = bridge(color, down, .4, .15 * S);
        break;
    }
    case 41: {
        st.y = 1. - st.y;
        float s = .25 * C*1.3;
        float t1 = polySDF(st + vec2(.0, .175), 3);
        float t2 = polySDF(st + vec2(.1, .0), 3);
        float t3 = polySDF(st - vec2(.1, .0), 3);
        color = stroke(t1, s, .08) + stroke(t2, s, .08) + stroke(t3, s, .08);
        float bridges = mix(mix(t1, t2, step(.5, st.y)), mix(t3, t2, step(.5, st.y)), step(.5, st.x));
        color = bridge(color, bridges, s, .08);
        break;
    }
    case 42: {
        float n = 12.;
        float a = TAU / n;
        for (float i = 0.; i < n; i++) {
            vec2 xy = rotate(st, a * i);
            xy.y -= .189;
            float vsc = vesicaSDF(xy, .3);
            color *= 1. - stroke(vsc, .45 * S, .1) * sstep(.5, xy.y);
            color += stroke(vsc, .45 * S, .05);
        }
        break;
    }
    case 43: {
        float n = 3.;
        float a = TAU / n;
        for (float i = 0.; i < n * 2.; i++) {
            vec2 xy = rotate(st, a * i);
            xy.y -= .09;
            float vsc = vesicaSDF(xy, .3);
            color = mix(
                color + stroke(vsc, .5, .1*S),
                mix(color, bridge(color, vsc, .5, .1*S), step(xy.x, .5) - step(xy.y, .4)),
                step(3., i)
            );
        }
        break;
    }
    case 44: {
        float star = starSDF(st, 8, .063);
        color += fill(star, 1.22);
        float n = 8.;
        float a = TAU / n;
        for (float i = 0.; i < n; i++) {
            vec2 xy = rotate(st, 0.39 + a * i);
            xy = scale(xy, vec2(1., .72) * S);
            xy.y -= .125;
            color *= sstep(.235, rhombSDF(xy));
        }
        break;
    }
    case 45: {
        st -= .5;
        float r = dot(st, st);
        float a = atan(st.y, st.x) / PI;
        vec2 uv = vec2(a, r);
        vec2 grid = vec2(5., log(r) * 20. * S);
        vec2 uv_i = floor(uv * grid);
        uv.x += .5 * mod(uv_i.y, 2.);
        vec2 uv_f = fract(uv * grid);
        float shape = rhombSDF(uv_f);
        color += fill(shape, .9) * sstep(.75, 1. - r);
        break;
    }
    case 46: {
        color = fill(flowerSDF(rotate(st, -iTime/4.), 5), .25*C);
        color -= sstep(.95, starSDF(rotate(st, 0.628 - iTime/4.), 5, .1*S));
        color = clamp(color, 0., 1.);
        float circle = circleSDF(st);
        color -= stroke(circle, .1, .05);
        color += stroke(circle, .8, .07);
        break;
    }
    case 47: { color = sstep(.5, spiralSDF(rotate(st, iTime/2.), .13 * S)); break; }
    case 48: { color = 1.; break; }
    case 49: {
        st = rotate(st, -iTime/4.);
        float d = .15;
        float r = .3 * S;
        color = fill(circleSDF(st - vec2(cos(TAU / 3.), sin(TAU / 3.)) * d), r);
        color += fill(circleSDF(st - vec2(cos(TAU / 3. * 2.), sin(TAU / 3. * 2.)) * d), r);
        color += fill(circleSDF(st - vec2(d, 0.)), r);
        st = st.yx;
        st.y = 1. - st.y;
        color *= 1. - fill(triSDF(st-vec2(0, .02)), .13);
        color += stroke(circleSDF(st), .8, .08);
        break;
    }
    }
    return color;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec2 count = vec2(10, 5);
    float tileW = iResolution.x / count.x;
    float tileH = iResolution.y / count.y;
    float tileAspectRatio = tileH / tileW;
    vec2 tileXY = floor(uv * count);
    vec2 st = vec2(
        uv.x * count.x - tileXY.x,
        (uv.y * count.y - tileXY.y - 0.5) * tileAspectRatio + .5
    );
    vec2 gridBars = clamp(cos(uv * TAU * count) * 10. - 9.8, 0., 1.);
    float grid = max(gridBars.x, gridBars.y);
    float color = draw(st, tileXY, count);
    color = clamp(color + grid * 0.5, 0., 1.);
    fragColor = vec4(vec3(color), 1.0);
}
