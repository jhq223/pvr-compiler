#version 100
#define SOURCE_KIND 2
#define STYLE 1
#ifndef SOURCE_KIND
#define SOURCE_KIND 0
#endif
#ifndef STYLE
#define STYLE 0
#endif
precision highp float;
varying vec2 v_uv;
varying vec4 v_color,v_window;
uniform sampler2D u_image,u_chroma_u,u_chroma_v;
uniform vec2 u_dimensions,u_output_size,u_repeat;
uniform vec4 u_appearance[9],u_video[4];
uniform float u_video_kind,u_image_flip,u_coverage;
vec3 linear_srgb(vec3 x){return mix(pow((x+0.055)/1.055,vec3(2.4)),x/12.92,step(x,vec3(0.04045)));}
vec3 encoded_srgb(vec3 x){return mix(1.055*pow(max(x,vec3(0.0)),vec3(1.0/2.4))-0.055,12.92*x,step(x,vec3(0.0031308)));}
vec2 flip_uv(vec2 uv){if(u_image_flip>0.5)uv.y=1.0-uv.y;return uv;}
vec2 source_uv(){
 vec2 low=v_window.xy,span=abs(v_window.zw),half_texel=0.5/u_dimensions;
 vec2 high=vec2(1.0)-half_texel;
 if(u_appearance[4].w>=3.0)high.y=u_appearance[8].y*0.5-half_texel.y;
 vec2 uv=v_uv;
 if(any(lessThanEqual(span,vec2(0.0)))){
  if(u_repeat.x>0.5)uv.x=fract(uv.x);if(u_repeat.y>0.5)uv.y=fract(uv.y);
  if(u_appearance[4].w>=3.0)uv=clamp(uv,half_texel,high);
 }else{
  if(v_window.z<0.0)uv.x=low.x+fract((uv.x-low.x)/span.x)*span.x;
  if(v_window.w<0.0)uv.y=low.y+fract((uv.y-low.y)/span.y)*span.y;
  vec2 inset=min(half_texel,span*0.5);
  uv=clamp(clamp(uv,low+inset,low+span-inset),half_texel,high);
 }
 return uv;
}
#if SOURCE_KIND == 3
float encoded_red(vec4 color){
 float value=0.0;
 if(color.a>0.0){
  float red=max(color.r/color.a,0.0);
  if(u_appearance[4].w==4.0)value=red;
  else value=red<=0.0031308?12.92*red:1.055*pow(red,1.0/2.4)-0.055;
 }
 return clamp(value,0.0,1.0);
}
float alpha_sample(vec2 uv){
 vec2 pixel=uv*u_dimensions-0.5,base=floor(pixel),t=fract(pixel);
 vec2 a=(clamp(base,vec2(0.0),u_dimensions-1.0)+0.5)/u_dimensions;
 vec2 b=(clamp(base+1.0,vec2(0.0),u_dimensions-1.0)+0.5)/u_dimensions;
 return mix(mix(encoded_red(texture2D(u_image,flip_uv(a))),encoded_red(texture2D(u_image,flip_uv(vec2(b.x,a.y)))),t.x),mix(encoded_red(texture2D(u_image,flip_uv(vec2(a.x,b.y)))),encoded_red(texture2D(u_image,flip_uv(b))),t.x),t.y);
}
#endif
#if SOURCE_KIND == 2
vec2 chroma(vec2 uv){
 vec4 sample=texture2D(u_chroma_u,uv);vec2 value;
 if(u_video_kind>1.5)value=sample.ra;
 else value=vec2(sample.r,texture2D(u_chroma_v,uv).r);
 return value;
}
#endif
vec4 source(vec2 uv){
#if SOURCE_KIND == 2
 vec4 yuv=vec4(texture2D(u_image,uv).r,chroma(uv),1.0);
 vec3 rgb=clamp(vec3(dot(u_video[0],yuv),dot(u_video[1],yuv),dot(u_video[2],yuv)),0.0,1.0);
 vec3 linear=linear_srgb(rgb);
 if(u_video[3].x<0.5)linear=mix(pow((rgb+0.099)/1.099,vec3(1.0/0.45)),rgb/4.5,step(rgb,vec3(0.081)));
 float alpha=1.0;
 if(u_appearance[4].w==3.0){vec2 at=uv+vec2(0.0,0.5);alpha=clamp(dot(u_video[0],vec4(texture2D(u_image,at).r,chroma(at),1.0)),0.0,1.0);}
 return vec4(linear*alpha,alpha);
#elif SOURCE_KIND == 1
 float alpha=texture2D(u_image,uv).r;return vec4(vec3(alpha),alpha);
#else
 vec4 color=texture2D(u_image,flip_uv(uv));
 if(u_appearance[8].z==1.0)color.a=1.0;
 if(u_appearance[8].z==2.0)color.rgb=min(color.rgb,vec3(color.a));
#if SOURCE_KIND == 3
 float alpha=alpha_sample(uv+vec2(0.0,u_appearance[8].y*0.5));color=vec4(color.a>0.0?color.rgb/color.a*alpha:vec3(0.0),alpha);
#endif
 return color;
#endif
}
#if STYLE == 2
vec4 gradient(vec2 position){
 float t=dot(u_appearance[4].xyz,vec3(position,1.0));
 if(u_appearance[5].x==1.0)t=fract(t);else if(u_appearance[5].x==2.0)t=1.0-abs(t-2.0*floor(t*0.5)-1.0);else t=clamp(t,0.0,1.0);
 float start=u_appearance[6].x,end=u_appearance[6].y;
 float u=start==end?step(end,t):clamp((t-start)/(end-start),0.0,1.0);
 vec4 color=mix(u_appearance[0],u_appearance[1],u);
 if(color.a==0.0)color=vec4(0.0);
 else if(u_appearance[5].y!=0.0||u_appearance[5].z==0.0){
  if(u_appearance[5].z!=0.0)color.rgb/=color.a;
  if(u_appearance[5].y!=0.0)color.rgb=sign(color.rgb)*linear_srgb(abs(color.rgb));
  color.rgb*=color.a;
 }
 return color;
}
#endif
void main(){
 vec4 color=source(source_uv())*v_color;
 vec2 position=vec2(gl_FragCoord.x,u_output_size.y-gl_FragCoord.y);
#if STYLE == 2
 color*=gradient(position);
#elif STYLE == 1
 if(u_appearance[5].w!=0.0){vec3 p=vec3(position,1.0);float x=clamp(dot(u_appearance[4].xyz,p),0.0,1.0),y=clamp(dot(u_appearance[5].xyz,p),0.0,1.0);color*=mix(mix(u_appearance[0],u_appearance[1],x),mix(u_appearance[3],u_appearance[2],x),y);}
 if(u_appearance[7].w!=0.0&&color.a>0.0){vec3 rgb=encoded_srgb(max(color.rgb/color.a,vec3(0.0)));float gray=dot(rgb,vec3(0.2126,0.7152,0.0722));rgb=vec3(gray)+u_appearance[6].w*(rgb-vec3(gray));rgb=clamp(rgb*u_appearance[6].xyz+u_appearance[7].xyz,0.0,1.0);color.rgb=linear_srgb(rgb)*color.a;}
#endif
 gl_FragColor=color;
}
