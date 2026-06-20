// Fullscreen-triangle vertex shader (HLSL → SPIR-V via glslang).
// No vertex buffer: positions are generated from SV_VertexID for verts 0,1,2.
float4 main(uint vid : SV_VertexID) : SV_Position {
    float2 uv = float2((vid << 1) & 2, vid & 2); // (0,0) (2,0) (0,2)
    return float4(uv * 2.0 - 1.0, 0.0, 1.0);     // covers the clip rect
}
