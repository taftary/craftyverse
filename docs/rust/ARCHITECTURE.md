# **ARCHITECTURE.md — Rust Game Project Structure**

This document defines the **architecture**, **crate layout**, and **module responsibilities** for this Rust game workspace.  
It is designed for long‑term maintainability, clear separation of concerns, and engine/game modularity.

---

## **1. Workspace Overview**

The project uses a **Cargo workspace** to separate the engine, game logic, and development tools.

```
my_game/
├── Cargo.toml                # Workspace root
├── crates/
│   ├── engine/               # Core engine (library crate)
│   ├── game/                 # Game logic (binary crate)
│   ├── tools/                # Optional: editors, asset pipeline
├── assets/                   # Game assets
└── tests/                    # Integration tests
```

### Why a workspace?
- Clear boundaries between engine and game  
- Reusable engine crate  
- Faster compilation via isolated crates  
- Cleaner dependency graph  
- Industry‑standard Rust architecture  

---

## **2. Engine Crate (`crates/engine`)**

The **engine** is a reusable library containing all core systems required by the game.

```
crates/engine/
├── src/
│   ├── lib.rs
│   ├── ecs/
│   │   ├── mod.rs
│   │   ├── components.rs
│   │   ├── systems.rs
│   ├── render/
│   │   ├── mod.rs
│   │   ├── pipeline.rs
│   │   ├── shaders.rs
│   │   ├── mesh.rs
│   ├── physics/
│   │   ├── mod.rs
│   │   ├── collision.rs
│   │   ├── spatial.rs
│   ├── assets/
│   │   ├── mod.rs
│   │   ├── loader.rs
│   │   ├── formats.rs
│   ├── prelude.rs
```

### Engine Responsibilities

#### **ECS (`ecs/`)**
- Entity creation & management  
- Component definitions  
- System execution  
- World storage  

#### **Rendering (`render/`)**
- wgpu pipeline setup  
- Shader management  
- Mesh & buffer handling  
- Render graph (optional)  

#### **Physics (`physics/`)**
- Collision detection  
- Spatial partitioning (QuadTree, BVH)  
- Movement & integration  

#### **Assets (`assets/`)**
- Texture/model/audio loading  
- Custom asset formats  
- Caching & hot‑reload (optional)  

#### **Prelude (`prelude.rs`)**
Convenient re‑exports for engine users.

---

## **3. Game Crate (`crates/game`)**

The **game** crate is the binary that uses the engine to implement game‑specific logic.

```
crates/game/
├── src/
│   ├── main.rs
│   ├── world.rs
│   ├── characters.rs
│   ├── ui.rs
│   ├── states.rs
```

### Game Responsibilities

#### **Startup (`main.rs`)**
- Initialize engine  
- Load assets  
- Enter game state machine  

#### **World (`world.rs`)**
- Map generation  
- Environment logic  
- Spawning rules  

#### **Characters (`characters.rs`)**
- Player logic  
- NPCs  
- AI behavior  

#### **UI (`ui.rs`)**
- HUD  
- Menus  
- Dialogs  

#### **States (`states.rs`)**
- Loading  
- Playing  
- Paused  
- Game over  

---

## **4. Tools Crate (`crates/tools`)**

Optional but recommended for serious development workflows.

```
crates/tools/
├── src/
│   ├── main.rs
│   ├── editor.rs
│   ├── asset_pipeline.rs
```

### Tools Responsibilities

#### **Editor (`editor.rs`)**
- Level editing  
- Entity placement  
- Tilemap editing  

#### **Asset Pipeline (`asset_pipeline.rs`)**
- Texture compression  
- Model preprocessing  
- Audio normalization  

---

## **5. Assets Directory**

```
assets/
├── textures/
├── models/
├── audio/
├── shaders/
```

### Asset Guidelines
- Use consistent naming conventions  
- Keep raw source files in `/assets_raw` (optional)  
- Preprocess assets via the tools crate  

---

## **6. Tests Directory**

```
tests/
├── integration.rs
```

### Testing Strategy
- Workspace‑level integration tests  
- Engine unit tests inside `crates/engine/src`  
- Game logic tests inside `crates/game/src`  

---

## **7. Design Principles**

### **Separation of Concerns**
Engine handles reusable systems; game handles content.

### **Feature‑Based Modules**
Organize by domain (ecs, render, physics), not file type.

### **Thin Game Crate**
Game should be mostly glue code + content.

### **Reusable Engine**
Engine must not depend on game logic.

### **Scalability**
Workspace architecture supports large projects.

---

## **8. Future Extensions**

- Networking crate (`crates/net`)  
- Scripting crate (`crates/script`)  
- Editor GUI using egui or iced  
- Asset hot‑reload system  
- Plugin architecture  
