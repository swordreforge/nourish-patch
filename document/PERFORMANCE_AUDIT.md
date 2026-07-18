# y5 合成器性能与内存占用审计报告

> 审计范围: 5020 个 Rust 文件 / ~37 万行代码
> 审计维度: 堆分配、并发与锁、GPU 与渲染、数据结构与算法
> 基线配置: stable toolchain, mold/lld 链接器, `-A warnings`
> 日期: 2026-07-18

---

## 目录

- [1. CRITICAL — 必须关注](#1-critical--必须关注)
  - [1.1 output\_key 每帧重复 format! 分配](#11-output_key-每帧重复-format-分配)
  - [1.2 Hit-test 全量克隆 Window 到 HashMap](#12-hit-test-全量克隆-window-到-hashmap)
  - [1.3 layout::compute() 每帧计算两次（相同参数）](#13-layoutcompute-每帧计算两次相同参数)
  - [1.4 X11 WindowManager 线性查找 O(n)](#14-x11-windowmanager-线性查找-ohn)
  - [1.5 Dmabuf 缓存永不清理失效条目](#15-dmabuf-缓存永不清理失效条目)
- [2. HIGH — 建议尽快处理](#2-high--建议尽快处理)
  - [2.1 CanvasSelect 每次点击重建 Vec](#21-canvasselect-每次点击重建-vec)
  - [2.2 Canvas scene 每帧全量克隆窗口](#22-canvas-scene-每帧全量克隆窗口)
  - [2.3 PropMap 短字符串用 String + SipHash](#23-propmap-短字符串用-string--siphash)
  - [2.4 GlesRenderer.extensions Vec 线性搜索](#24-glesrendererextensions-vec-线性搜索)
  - [2.5 Staging buffer 每次上传 map/unmap](#25-staging-buffer-每次上传-mapunmap)
  - [2.6 每帧 N 次独立 descriptor set 分配](#26-每帧-n-次独立-descriptor-set-分配)
  - [2.7 默认同步路径 device\_wait\_idle N+1 次](#27-默认同步路径-device_wait_idle-n1-次)
  - [2.8 无 VkMemory 子分配器](#28-无-vkmemory-子分配器)
- [3. MEDIUM — 可优化](#3-medium--可优化)
  - [3.1 单线程数据使用 Mutex](#31-单线程数据使用-mutex)
  - [3.2 CaptureRegistry 每 entry 锁-释放-重锁](#32-captureregistry-每-entry-锁-释放-重锁)
  - [3.3 无界 mpsc::channel](#33-无界-mpscchannel)
  - [3.4 DrawOrder Vec 线性扫描](#34-draworder-vec-线性扫描)
  - [3.5 Space.elements Vec position() 查找](#35-spaceelements-vec-position-查找)
  - [3.6 GroupState Vec 按 UUID 查找](#36-groupstate-vec-按-uuid-查找)
  - [3.7 layout::compute() 在 context builder 中重复调用](#37-layoutcompute-在-context-builder-中重复调用)
  - [3.8 Pipeline cache 不持久化](#38-pipeline-cache-不持久化)
  - [3.9 FullscreenPass 管线绕过 pipeline cache](#39-fullscreenpass-管线绕过-pipeline-cache)
  - [3.10 HDR UBO 独立 VkDeviceMemory](#310-hdr-ubo-独立-vkdevicememory)
  - [3.11 未使用的 broadcast channel](#311-未使用的-broadcast-channel)
  - [3.12 Cursor 同步磁盘 I/O](#312-cursor-同步磁盘-io)
- [4. LOW — 次要优化](#4-low--次要优化)
- [5. 优化优先级矩阵](#5-优化优先级矩阵)
- [6. 附录: 审计方法说明](#6-附录审计方法说明)

---

## 1. CRITICAL — 必须关注

### 1.1 output_key 每帧重复 format! 分配

**文件**

- `compositor.orchestration/orchestration.core/core.state/state.base/state.rs:66`
- `compositor.expansion/compositor.y5/y5.surface/surface.interface/interface.core/hit.rs:96`
- `compositor.expansion/compositor.y5/y5.surface/surface.interface/interface.core/hit.rs:107`

**问题**

`format!("{} {} {}", make, model, serial)` 在每次渲染帧、每次输入事件、每次命中测试时调用，每次都堆分配一个新的 `String`。该字符串用作 output 的唯一键，频繁比较但内容极少变化。

**调用频率**

- 渲染路径: 每帧 × 每输出
- 输入路径: 每次鼠标移动 × 命中测试

**影响**

每次调用触发堆分配 + 堆释放，在 60fps 下每秒产生数十到数百次无意义的 String 分配。

**建议**

将 `output_key` 缓存在 `Output` 的用户数据或一个 `OnceCell` 中，在模式切换时更新一次即可：

```rust
// 方案 A: 缓存在 Output 的 UserData 中（推荐）
// 在 output 热插拔/模式变更时写入一次
output.user_data().insert_if_missing(|| CachedOutputKey::new(format!("{} {} {}", p.make, p.model, p.serial_number)));

// 方案 B: 用 SmolStr 内联小字符串，避免堆分配
use smol_str::SmolStr;
let key: SmolStr = format!("{} {} {}", p.make, p.model, p.serial_number).into();
// SmolStr 对 <= 22 字节的字符串做栈内联，零堆分配
```

---

### 1.2 Hit-test 全量克隆 Window 到 HashMap

**文件**

- `compositor.expansion/compositor.y5/y5.surface/surface.interface/interface.core/hit.rs:644-645`
- `compositor.expansion/compositor.y5/y5.surface/surface.interface/interface.core/hit.rs:649,658`

**问题**

每次命中测试（每次鼠标移动）执行：

```rust
let by_uuid: HashMap<Uuid, Window> = space.elements()
    .filter_map(|w| { let u = w.uuid()?; Some((u, w.clone())) })
    .collect();
let in_order: HashSet<Uuid> = space.elements()
    .filter_map(|w| w.uuid())
    .collect();
```

将**所有窗口**克隆到 `HashMap<Uuid, Window>` 和 `HashSet<Uuid>` 中。对于有 N 个窗口的合成器，每次鼠标事件触发 N 次 `Window` 克隆 + 两次 HashMap/HashSet 构建。

**调用频率**: 每次 `PointerMotion` 事件（鼠标移动频率可达 1000+ 次/秒）

**影响**

- N 次 `Window` 结构体克隆（每个 Window 含多个 String/Vec 字段）
- 两次哈希表构建 + 两次堆分配
- 鼠标移动时的延迟抖动

**建议**

直接迭代 `space.elements()`，在需要 UUID 查找时用轻量级 `HashSet<Uuid>` 做成员检查，不要克隆整个 Window：

```rust
// 方案: 仅收集 UUID 集合，不克隆 Window
let drawn_uuids: HashSet<Uuid> = space.elements()
    .filter_map(|w| w.uuid())
    .collect();

// 需要按 UUID 查找时，重新迭代 space.elements() 做 O(1) 成员检查
// 或者只维护一个 Vec<(Uuid, &Window)> 的借用视图
```

---

### 1.3 layout::compute() 每帧计算两次（相同参数）

**文件**

- `compositor.orchestration/orchestration.draw/draw.scene/scene.frame/scene.rs:200`（`iced_panes()` 中）
- `compositor.orchestration/orchestration.draw/draw.scene/scene.frame/scene.rs:424`（content band 中）

**问题**

代码注释承认了重复：

> recomputes the same `layout::compute` the content band uses; the duplication
> is deliberate — backing (de)allocation needs the GLES renderer and must run
> in `prepare()`, before the renderer-agnostic `scene()` builds the content band.

两次调用使用完全相同的参数 (`viewports`, `bounds`)，viewport 树和 output bounds 在两次调用间不变。

**调用频率**: 每帧 × 每输出

**影响**

每次 `layout::compute()` 执行完整的布局树遍历，是帧内较重的计算之一。重复执行直接浪费 CPU 时间。

**建议**

在 `prepare()` 阶段缓存 `Computed` 结果，传递给 `scene()` 复用：

```rust
// 在 frame state 中增加缓存字段
struct FrameState {
    cached_layout: Option<Computed>,
}

// prepare() 中计算一次并缓存
state.cached_layout = Some(layout::compute(&viewports, &bounds));

// scene() 中直接使用缓存
let computed = state.cached_layout.as_ref().unwrap();
```

---

### 1.4 X11 WindowManager 线性查找 O(n)

**文件**

- `vendor/smithay/src/xwayland/xwm/mod.rs`（~20 处调用）

**问题**

`windows: Vec<X11Surface>` 在约 20 个位置被线性扫描：

```rust
self.windows.iter().find(|s| s.window_id() == *w)     // O(n) per call
self.windows.iter().any(|x| x.window_id() == n.event)  // O(n) per call
```

关键位置包括:

| 行号 | 上下文 |
|------|--------|
| 1060 | `MapRequest` 处理 |
| 1105 | 嵌套循环中 — O(n²) |
| 1580, 1702 | `ConfigureRequest` |
| 1789, 1807 | `UnmapNotify` |
| 2285, 2340 | `ClientMessage` |
| 2448, 2474 | `PropertyNotify` |
| 2585, 2612, 2629, 2673 | 其他 X11 事件 |

行 1105 的嵌套循环更是 O(ordering × windows) 的三重嵌套：

```rust
for relatable in order {
    let stacking_ordered_elems: Vec<&X11Surface> = self
        .client_list_stacking
        .iter()
        .filter_map(|w| self.windows.iter().find(|s| s.window_id() == *w))
        .collect();  // 对每个 order 元素扫描整个 windows Vec
    let pos = stacking_ordered_elems.iter().position(|w| relatable.is_window(w));
}
```

**调用频率**: 每个 X11 事件

**影响**

运行 X11 应用时，事件处理延迟随窗口数量线性增长。嵌套循环场景为 O(n²) 甚至 O(n³)。

**建议**

添加一个辅助索引，在 `push` / `remove` 时维护：

```rust
struct XwmState {
    windows: Vec<X11Surface>,
    window_index: HashMap<X11Window, usize>,  // 新增: window_id -> Vec 索引
}

impl XwmState {
    fn find_window(&self, id: X11Window) -> Option<&X11Surface> {
        self.window_index.get(&id)
            .and_then(|&idx| self.windows.get(idx))
    }
}
```

---

### 1.5 Dmabuf 缓存永不清理失效条目

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.renderer/renderer.core/core.base/renderer/import.rs:84-111`

**问题**

`dmabuf_cache: HashMap<WeakDmabuf, VulkanTexture>` 中，当客户端销毁 buffer 后，`WeakDmabuf` 变为悬空弱引用，但缓存条目（含导入的 `VkImage` + `VkDeviceMemory`）持续存在直到渲染器销毁。无定期清理或 `upgrade()` 检查。

**调用频率**: 持续累积（客户端创建/销毁 buffer 时）

**影响**

GPU 内存泄漏。对于运行大量客户端的长时间合成器会话，泄漏量与历史上所有不同 dmabuf 的总数成正比。每个条目包含一个 `VkImage` 和对应的 `VkDeviceMemory`。

**建议**

定期（如每秒或每 N 帧）扫描缓存，移除 `Weak::upgrade()` 失败的条目：

```rust
fn sweep_dmabuf_cache(cache: &mut HashMap<WeakDmabuf, VulkanTexture>, device: &Device) {
    cache.retain(|weak, tex| {
        if weak.upgrade().is_none() {
            // 释放 VkImage + VkDeviceMemory
            tex.destroy(device);
            false
        } else {
            true
        }
    });
}
```

---

## 2. HIGH — 建议尽快处理

### 2.1 CanvasSelect 每次点击重建 Vec

**文件**

- `compositor.expansion/compositor.y5/y5.select/select.state/state.base/select.rs:35,43,69,106,150`

**问题**

`CanvasSelect` 的每次操作（`clear`, `erase_uuid`, `set`, `append`, `exact`）都调用 `self.clone()`，克隆内部的 `Vec<Arc<Window>>`。`Arc` 本身廉价，但 `Vec` 克隆导致堆重分配。

```rust
pub fn clear(&mut self) {
    self.clone();  // 克隆整个 Vec<Arc<Window>>
    self.0 = Selection::default();
}
```

此外，`get()` 等方法每次调用 `Arc::new(window)`，即使调用方可能已经持有 `Arc<Window>`。

**调用频率**: 每次用户点击/选择操作

**建议**

改为就地修改：

```rust
pub fn clear(&mut self) {
    self.0 = Selection::default();  // 直接赋值，不 clone
}

pub fn set(&mut self, window: Arc<Window>) {
    self.0 = Selection::Single(window);  // 就地覆盖
}
```

---

### 2.2 Canvas scene 每帧全量克隆窗口

**文件**

- `compositor.expansion/compositor.y5/y5.canvas/canvas.draw/draw.scene/scene.rs:40-42,64`

**问题**

与 1.2 相同模式 — 每帧每 pane 克隆所有窗口到 HashMap + HashSet：

```rust
let by_uuid: HashMap<Uuid, Window> = space.elements()
    .filter_map(|w| w.clone())
    .collect();
```

**建议**: 同 1.2 — 迭代 space.elements() 直接使用，不克隆。

---

### 2.3 PropMap 短字符串用 String + SipHash

**文件**

- `compositor.kernel/kernel.scanout/scanout.commit/commit.build/build.base/build.rs:28`

**问题**

```rust
pub struct PropMap(HashMap<String, property::Handle>);
```

KMS 属性名是约 30 个短固定字符串（`"MODE_ID"`, `"CONNECTOR_ID"` 等），每个 `String` 堆分配 24+ 字节（ptr+len+cap），配合 SipHash 开销。

**建议**

```rust
// 方案 A: SmolStr — 对 <= 22 字节的字符串做栈内联
pub struct PropMap(HashMap<SmolStr, property::Handle>);

// 方案 B: 固定映射 — 既然属性名固定且数量少
pub struct PropMap {
    by_name: HashMap<&'static str, property::Handle>,
}
```

---

### 2.4 GlesRenderer.extensions Vec 线性搜索

**文件**

- `vendor/smithay/src/backend/renderer/gles/mod.rs:387`
- 运行时搜索位置: 行 1206, 1259, 1873

**问题**

```rust
pub(crate) extensions: Vec<String>,
// 运行时:
if self.extensions.iter().any(|ext| ext == "...") { ... }
```

GL 扩展串在初始化后不变，但每次检查都做线性搜索。

**建议**

初始化时构建 `HashSet<&'static str>`：

```rust
extensions: HashSet<&'static str>,  // init 时 from_iter
// 运行时:
if self.extensions.contains("...") { ... }
```

---

### 2.5 Staging buffer 每次上传 map/unmap

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.memory/memory.upload/upload.base/upload.rs:93-97`

**问题**

每次 SHM surface 上传都调用 `map_memory` + `unmap_memory`。Vulkan 最佳实践是持久映射 staging buffer。

**影响**

每次 map/unmap 在驱动层产生额外开销，对于每帧处理数十个 SHM surface 的合成器，累积开销可观。

**建议**

Staging buffer 创建时持久映射，上传时直接 memcpy：

```rust
struct StagingBuffer {
    memory: *mut u8,  // 创建时 map，生命周期内不 unmap
    // ...
}

fn stage(&mut self, data: &[u8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), self.memory.add(self.offset), data.len());
    }
    self.offset += data.len();
}
```

---

### 2.6 每帧 N 次独立 descriptor set 分配

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.renderer/renderer.core/core.base/renderer/submit.rs:246-256`

**问题**

每个纹理绘制单独调用 `allocate_descriptor_sets` + `update_descriptor_sets`：

```rust
for textured_draw in &textured_draws {
    let desc_set = device.allocate_descriptor_sets(&alloc_info)?;  // N 次
    device.update_descriptor_sets(&writes, &[]);                    // N 次
}
```

**建议**

批量分配或使用 `VK_KHR_push_descriptor` 扩展完全避免 descriptor pool 分配。

---

### 2.7 默认同步路径 device\_wait\_idle N+1 次

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.renderer/renderer.core/core.base/renderer/submit.rs:334-349`

**问题**

默认提交路径在每次 `device_wait_idle()` 后提交帧。此外:

- `transition_to_sampled()` (`import.rs:60`) — 每个新 dmabuf 导入一次
- `one_time()` (`upload.rs:158`) — 每次 SHM 导入/更新一次
- `record_capture_blits()` (`blit.rs:324`) — 每次 capture blit 一次

一帧中 N 个新导入 + M 个 SHM 更新 = N+M+1 次 `device_wait_idle`。

**影响**

完全序列化 CPU 和 GPU，帧率上限被 GPU 执行时间严格限制。Infence 路径（`COMPOSITOR_RENDERER_SYNC=infence`）正确实现了异步提交，但非默认。

**建议**

- 确保 Infence 路径在生产构建中默认启用
- 或在同步路径中将所有 GPU 操作合并到单个 command buffer，提交一次后等一次

---

### 2.8 无 VkMemory 子分配器

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.memory/memory.alloc/`
- `compositor.kernel/kernel.vulkan/vulkan.memory/memory.import/`
- `compositor.kernel/kernel.vulkan/vulkan.memory/memory.target/`
- `compositor.kernel/kernel.vulkan/vulkan.memory/memory.upload/`
- `compositor.kernel/kernel.vulkan/vulkan.renderer/.../mipgen.rs`

**问题**

每个 `VkImage` 和 `VkBuffer` 使用独立的 `vkAllocateMemory`。无 ring allocator 或 VMA 风格子分配。每个 `VkImage` 获得独立的 `VkDeviceMemory` 分配。

在 Intel 等驱动上，设备 heap 限制通常为 4096 个分配。随着活跃纹理增长，可能耗尽限制。

**影响**

- 每次分配在驱动层产生固定开销
- 可能触及驱动 heap 分配上限
- `mipgen.rs` 中 `MAX_SURFACES=32` 个独立分配
- `dmabuf_cache` 无限增长（见 1.5）

**建议**

引入子分配器，将瞬态资源（staging buffer、mip images）分配到 ring buffer 中，减少总分配次数。

---

## 3. MEDIUM — 可优化

### 3.1 单线程数据使用 Mutex

**文件**

- `compositor.orchestration/orchestration.seat/seat.pointer/pointer.element/element.rs:28,55`
- `compositor.expansion/compositor.y5/y5.camera/camera.transform/transform.translate/slot.rs:33,75`
- `compositor.kernel/kernel.graphic/graphic.color/color.surface/surface.base/lib.rs:19,24,29`

**问题**

以下数据存储在 `Mutex` 中，但仅从单线程 calloop 事件循环访问：

| 类型 | 位置 | 每帧调用次数 |
|------|------|-------------|
| `Mutex<AnimState>` | `element.rs:28` | 每帧 × 每指针渲染元素 |
| `Mutex<Option<Slot>>` | `slot.rs:33` | 每帧 × 每窗口 |
| `Mutex<Option<ResizePending>>` | `slot.rs:75` | 每帧 × 每窗口 |
| `Mutex<Option<SurfaceHdr>>` | `surface.base/lib.rs:19` | 每帧 × 每表面 |

`std::sync::Mutex` 即使无竞争也会触发 futex 系统调用来验证锁状态。在 60fps 下，50 个窗口 × 每帧多次访问 = 每秒数百次不必要的 futex。

**建议**

```rust
// 将 Mutex 替换为 Cell/RefCell（单线程安全）
struct ExpectedSize {
    inner: Cell<Option<Slot>>,  // 零开销，无系统调用
}
```

---

### 3.2 CaptureRegistry 每 entry 锁-释放-重锁

**文件**

- `compositor.expansion/compositor.y5/y5.graphic/graphic.capture/capture.registry/registry.rs:269-299`

**问题**

```rust
fn tick(&mut self) {
    let entries: Vec<_> = self.inner.lock().unwrap().drain_entries();
    for entry in entries {
        // GPU blit（耗时操作）
        blit(entry.texture, ...);
        self.inner.lock().unwrap().update(entry);  // 重锁
    }
}
```

每个 capture entry 触发一次锁-释放-重锁循环。N 个 capture = N 次 Mutex lock/unlock。

**建议**

在单次加锁下预拷贝所有 entry 数据到局部 Vec，释放锁后遍历局部 Vec 做 GPU blit：

```rust
fn tick(&mut self) {
    let snapshot: Vec<EntrySnapshot> = {
        let inner = self.inner.lock().unwrap();
        inner.entries.iter().map(|e| e.snapshot()).collect()
    };
    // 无锁状态下执行 GPU 操作
    for snap in snapshot {
        blit(snap.texture, ...);
    }
}
```

---

### 3.3 无界 mpsc::channel

**文件**

- `compositor.expansion/compositor.y5/y5.surface/surface.state/state.base/state.rs:13,21`

**问题**

Surface 消息通道使用无界 `mpsc::channel()`。UI 消息处理器可能在帧之间发送大量消息，累积到下一个 `try_recv()` 循环前无背压。

**建议**

```rust
let (tx, rx) = std::sync::mpsc::sync_channel(64);  // 有界，64 消息
```

---

### 3.4 DrawOrder Vec 线性扫描

**文件**

- `compositor.support/support.world/world.order/order.track/track.base/base.rs:33`

**问题**

`entries: Vec<(ComponentId, DrawLayer, OrderKey)>` 在 `insert_top`, `key`, `reassign`, `raise`, `remove` 中被 `.iter().find()` 和 `.iter().any()` 线性扫描。每次 draw 操作调用。

**建议**

维护并行 `HashMap<ComponentId, usize>` 索引。

---

### 3.5 Space.elements Vec position() 查找

**文件**

- `vendor/smithay/src/desktop/space/mod.rs:129-248`

**问题**

`elements: Vec<InnerElement<E>>` 在 `map_element`, `create_inner_element`, `raise_element`, `lower_element`, `refresh` 中使用 `.iter().position()` 查找。

**建议**

维护 `HashMap<ElementId, usize>` 索引。

---

### 3.6 GroupState Vec 按 UUID 查找

**文件**

- `compositor.expansion/compositor.y5/y5.group/group.state/state.base/state.rs:96`

**问题**

```rust
let group = 'group: { self.group.iter().position(|w| w.id == group_uuid) };
```

`Vec<Group>` 用 UUID 查找，每次 group 操作 O(n)。

**建议**: 改为 `HashMap<Uuid, Group>`。

---

### 3.7 layout::compute() 在 context builder 中重复调用

**文件**

- `compositor.orchestration/orchestration.core/core.state/state.base/state.rs:742,791,842`

**问题**

三个 context builder 各自调用 `layout::compute()` 并线性搜索 regions：

```rust
fn pointer_context(&self) -> ... {
    let computed = layout::compute(...);  // 完整 layout 计算
    let region = computed.regions.iter().find(|r| ...);  // 线性搜索
}
```

**建议**: 缓存 computed 结果，用 HashMap 索引 region 查找。

---

### 3.8 Pipeline cache 不持久化

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.pipeline/pipeline.cache/cache.base/cache.rs:13-21`

**问题**

Pipeline cache 用空初始数据创建，从不序列化到磁盘。每次冷启动都支付完整的驱动管线编译代价。

**建议**

在首次创建后序列化 cache data 到文件，下次启动时从文件反序列化作为初始数据：

```rust
// 启动时
let cache_data = std::fs::read("pipeline_cache.bin").unwrap_or_default();
let cache = device.create_pipeline_cache(&PipelineCacheCreateInfo {
    initial_data: &cache_data,
    ..Default::default()
})?;

// 关闭时
let data = device.get_pipeline_cache_data(&cache);
std::fs::write("pipeline_cache.bin", data)?;
```

---

### 3.9 FullscreenPass 管线绕过 pipeline cache

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.pipeline/pipeline.fullscreen/fullscreen.base/fullscreen.rs:116`

**问题**

```rust
create_graphics_pipelines(device, vk::PipelineCache::null(), &info, None)
//                                                       ^^^^^^^^
//                                                       传 null，绕过缓存
```

Composite 和 HDR 管线正确使用缓存，但 FullscreenPass（视差背景、shader 效果）不使用。

**建议**: 传入 renderer 的 pipeline cache handle。

---

### 3.10 HDR UBO 独立 VkDeviceMemory

**文件**

- `compositor.kernel/kernel.vulkan/vulkan.pipeline/pipeline.hdr/hdr.base/hdr.rs:242-250`

**问题**

为 48 字节的 HDR tuning UBO 分配独立 `vkAllocateMemory`。驱动最小分配粒度通常为 64KB。

**建议**: 使用 push constants（< 128 字节，多数驱动支持）或将 UBO 放入共享分配。

---

### 3.11 未使用的 broadcast channel

**文件**

- `compositor.expansion/compositor.remote/remote.transport/transport.server/server.base/transport.rs:27-30`

**问题**

```rust
let (_broadcast_tx, _broadcast_rx) = tokio::sync::broadcast::channel::<Message>(100);
// receiver 立即 drop，分配了 100 slot 但无人消费
```

**建议**: 移除或接入实际的订阅者。

---

### 3.12 Cursor 同步磁盘 I/O

**文件**

- `compositor.orchestration/orchestration.seat/seat.pointer/pointer.texture/pointer_load.rs:53,62`

**问题**

首次访问每个 cursor 名称时，在渲染线程上执行同步 `File::open` + `read_to_end`。冷缓存 miss 会导致帧卡顿。

**建议**

启动时预加载所有常用 cursor（`default`, `pointer`, `text`, `grab`, `not-allowed` 等）。

---

## 4. LOW — 次要优化

| # | 文件 | 问题 | 建议 |
|---|------|------|------|
| L1 | `scene.rs:50-56` | `uuid_surface` HashMap 每帧从头构建，无 `with_capacity` | `HashMap::with_capacity(space.elements().count())` |
| L2 | `node.rs:71-72` | `Plan::default()` 无容量预分配 | `Vec::with_capacity(32)` |
| L3 | `node.rs:106-107` | `lower()` 的 elements/meta Vec 无容量 | `Vec::with_capacity(self.nodes.len() * 2)` |
| L4 | `node.rs:28-33` | `SurfaceNode` 有 4 字节尾部填充 | 重排字段: `surface, location, alpha, scale` |
| L5 | `node.rs:35-60` | `DrawNode<R>` 枚举大小由最大变体决定 | 对大变体用 `Box` 包装 |
| L6 | `select.rs:95-157` | `get()` 等方法每次 `Arc::new(window)` | 接受 `Arc<Window>` 参数 |
| L7 | `navigator/interface.rs:205-270` | 多处 `.cloned().collect()` 克隆全部窗口 | 迭代使用借用 |
| L8 | `buffers.rs:7` | `drain(..).collect()` 可改为直接迭代 drain | `for msg in self.drain(..)` |
| L9 | `persist.engine/base.rs:29,48` | 全局 Mutex 在 try_recv 循环期间持有 | 缩小锁粒度 |
| L10 | `capture.registry/handle.rs:40-61` | CaptureHandle 方法每帧做 weak upgrade + lock | 缓存结果或合并 accessor |
| L11 | `fps.rs:116,153` | HashMap key 含 String，每帧 clone | key 改为 `Uuid` 或 `SmolStr` |
| L12 | `state.rs:35,43,69,106,150` | `CanvasSelect::clone()` Vec 重分配 | 就地修改 |
| L13 | `draw.node/node.rs:71` | Plan nodes Vec 无容量 | `Vec::with_capacity(32)` |
| L14 | `execute.rs:388` | `visible_window: Vec<_> = Vec::new()` 无容量 | `Vec::with_capacity(16)` |
| L15 | `canvas.scene.rs:30-31` | content/visible_windows Vec 无容量 | 按实际大小预分配 |

---

## 5. 优化优先级矩阵

| 优先级 | 项目 | 类型 | 影响范围 | 预估收益 |
|--------|------|------|----------|----------|
| **P0** | 缓存 output_key (#1.1) | 内存 | 每帧每事件 | 消除数十~数百次/秒 String 分配 |
| **P0** | Hit-test 避免全量克隆 (#1.2, #2.2) | 内存+CPU | 每次鼠标移动 | 消除 O(n) 克隆 + 哈希表构建 |
| **P0** | 缓存 layout::compute (#1.3) | CPU | 每帧 | 减少 1 次完整 layout 计算 |
| **P0** | Dmabuf 缓存清理 (#1.5) | 内存 | 持续 | 防止 GPU 内存无限泄漏 |
| **P1** | X11 Vec→HashMap (#1.4) | CPU | X11 事件 | O(n)→O(1) 查找 |
| **P1** | 就地修改 CanvasSelect (#2.1) | 内存 | 用户操作 | 消除每次点击 Vec 重分配 |
| **P1** | Mutex→Cell (#3.1) | CPU | 每帧 | 减少数百次/秒 futex |
| **P1** | VkMemory 子分配 (#2.8) | GPU 内存 | 持续 | 减少分配次数，防 heap 耗尽 |
| **P2** | Staging buffer 持久映射 (#2.5) | GPU | 每帧每上传 | 减少驱动 map/unmap 开销 |
| **P2** | Pipeline cache 持久化 (#3.8) | 启动 | 启动时 | 加快冷启动 |
| **P2** | 批量 descriptor set (#2.6) | GPU | 每帧 | 减少 API 调用次数 |
| **P2** | device_wait_idle 合并 (#2.7) | GPU | 每帧 | 允许 CPU/GPU 并行 |
| **P3** | 所有 LOW 项目 | 混合 | 低频/小范围 | 代码质量改善 |

---

## 6. 附录: 审计方法说明

本次审计从以下四个维度进行并行分析:

1. **堆分配审计**: 搜索 `.clone()`、`.collect()`、`format!`、`Vec::new()` 等模式，聚焦渲染和输入热路径
2. **并发与锁审计**: 分析 `Mutex`/`RwLock`/`Arc` 使用、channel 模式、`spawn` 模式
3. **GPU/渲染审计**: 分析 Vulkan 资源管理（内存分配、管线缓存、descriptor set、同步）
4. **数据结构审计**: 搜索线性查找、O(n²) 模式、字符串密集结构、枚举膨胀

所有发现均标注了文件路径和行号，可直接定位。优先级基于**调用频率 × 单次开销 × 影响范围**综合评估。
