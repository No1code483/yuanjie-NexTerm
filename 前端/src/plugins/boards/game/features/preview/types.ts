import { t } from "i18next";
export interface GameVec2 {
  x: number;
  y: number;
}
export interface GameBuilding {
  id: string;
  building_type: string;
  name: string;
  level: number;
  position: GameVec2;
  built_at: string;
  upgraded_at: string;
}
export interface GameTile {
  terrain: string;
  building: GameBuilding | null;
}
export interface GameCharacter {
  name: string;
  cultivation_stage: string;
  cultivation_progress: number;
  total_xp: number;
  spirit_stones: number;
  last_kb_entry_count: number;
  last_kb_category_count: number;
}
export interface GameWorld {
  id: string;
  player_name: string;
  width: number;
  height: number;
  grid: GameTile[][];
  character: GameCharacter;
  created_at: string;
  updated_at: string;
}
export interface BuildingCatalogItem {
  building_type: string;
  name: string;
  description: string;
  required_entries: number;
  required_categories: number;
  required_stage: string;
  spirit_stone_cost: number;
  max_level: number;
  level_names: string[];
}
export interface BuildingLevelUpResult {
  building: GameBuilding;
  spirit_stone_cost: number;
  new_level: number;
  level_name: string;
}
export interface RewardLog {
  id: string;
  world_id: string;
  source: string;
  reward_type: string;
  amount: number;
  created_at: string;
}
export interface KnowledgeStats {
  total_entries: number;
  total_categories: number;
  recent_activity_count: number;
}
export interface CultivationStageInfo {
  stage: string;
  name: string;
  xp_required: number;
  max_buildings: number;
  index: number;
}
export interface Achievement {
  id: string;
  name: string;
  description: string;
  icon: string;
  condition: (ctx: {
    world: GameWorld;
    catalog: BuildingCatalogItem[];
    kbStats: KnowledgeStats | null;
    stageOrder: (s: string) => number;
  }) => boolean;
  progress?: (ctx: {
    world: GameWorld;
    catalog: BuildingCatalogItem[];
    kbStats: KnowledgeStats | null;
  }) => {
    current: number;
    target: number;
  };
  unlocked?: boolean;
}
export interface RandomEvent {
  type: string;
  title: string;
  message: string;
  icon: string;
  reward?: {
    spirit_stones?: number;
    xp?: number;
  };
  penalty?: {
    spirit_stones?: number;
    building_damage?: number;
  };
}
export const BUILDING_PRODUCTION: Record<string, {
  spirit_stones: number;
  xp: number;
}> = {
  spirit_stone_mine: {
    spirit_stones: 12,
    xp: 3
  },
  meditation_room: {
    spirit_stones: 3,
    xp: 15
  },
  alchemy_lab: {
    spirit_stones: 8,
    xp: 5
  },
  library: {
    spirit_stones: 2,
    xp: 10
  },
  treasure_vault: {
    spirit_stones: 18,
    xp: 1
  },
  formation_tower: {
    spirit_stones: 5,
    xp: 8
  },
  spirit_farm: {
    spirit_stones: 10,
    xp: 4
  },
  training_ground: {
    spirit_stones: 4,
    xp: 12
  }
};
export const BUILDING_SPECIAL_EFFECTS: Record<string, {
  label: string;
  desc: string;
}> = {
  spirit_stone_mine: {
    label: t("game.types.k1"),
    desc: t("game.types.k2")
  },
  meditation_room: {
    label: t("game.types.k3"),
    desc: t("game.types.k4")
  },
  alchemy_lab: {
    label: t("game.types.k5"),
    desc: t("game.types.k6")
  },
  library: {
    label: t("game.types.k7"),
    desc: t("game.types.k8")
  },
  treasure_vault: {
    label: t("game.types.k9"),
    desc: t("game.types.k10")
  },
  formation_tower: {
    label: t("game.types.k11"),
    desc: t("game.types.k12")
  },
  spirit_farm: {
    label: t("game.types.k13"),
    desc: t("game.types.k14")
  },
  training_ground: {
    label: t("game.types.k15"),
    desc: t("game.types.k16")
  }
};
export const ACHIEVEMENTS: Achievement[] = [{
  id: 'first_step',
  name: t("game.types.k17"),
  description: t("game.types.k18"),
  icon: '🌏',
  condition: () => true
}, {
  id: 'first_building',
  name: t("game.types.k19"),
  description: t("game.types.k20"),
  icon: '🏠',
  condition: ({
    world
  }) => world.grid.some(row => row.some(t => !!t.building))
}, {
  id: 'qi_refining',
  name: t("game.types.k21"),
  description: t("game.types.k22"),
  icon: '🌀',
  condition: ({
    world
  }) => stageOrder(world.character.cultivation_stage) >= 1
}, {
  id: 'spirit_tycoon',
  name: t("game.types.k23"),
  description: t("game.types.k24"),
  icon: '💎',
  condition: ({
    world
  }) => world.character.spirit_stones >= 100,
  progress: ({
    world
  }) => ({
    current: world.character.spirit_stones,
    target: 100
  })
}, {
  id: 'builder_initiate',
  name: t("game.types.k25"),
  description: t("game.types.k26"),
  icon: '🏗️',
  condition: ({
    world
  }) => {
    const types = new Set<string>();
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) types.add(t.building.building_type);
    }));
    return types.size >= 3;
  },
  progress: ({
    world
  }) => {
    const types = new Set<string>();
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) types.add(t.building.building_type);
    }));
    return {
      current: types.size,
      target: 3
    };
  }
}, {
  id: 'golden_core',
  name: t("game.types.k27"),
  description: t("game.types.k28"),
  icon: '🟡',
  condition: ({
    world
  }) => stageOrder(world.character.cultivation_stage) >= 3,
  progress: ({
    world
  }) => ({
    current: stageOrder(world.character.cultivation_stage),
    target: 3
  })
}, {
  id: 'master_builder',
  name: t("game.types.k29"),
  description: t("game.types.k30"),
  icon: '🏰',
  condition: ({
    world
  }) => {
    const types = new Set<string>();
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) types.add(t.building.building_type);
    }));
    return types.size >= 8;
  },
  progress: ({
    world
  }) => {
    const types = new Set<string>();
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) types.add(t.building.building_type);
    }));
    return {
      current: types.size,
      target: 8
    };
  }
}, {
  id: 'level_10',
  name: t("game.types.k31"),
  description: t("game.types.k32"),
  icon: '⬆️',
  condition: ({
    world
  }) => world.grid.some(row => row.some(t => !!t.building && t.building.level >= 10)),
  progress: ({
    world
  }) => {
    let maxLevel = 0;
    world.grid.forEach(row => row.forEach(t => {
      if (t.building && t.building.level > maxLevel) maxLevel = t.building.level;
    }));
    return {
      current: maxLevel,
      target: 10
    };
  }
}, {
  id: 'knowledge_power',
  name: t("game.types.k33"),
  description: t("game.types.k34"),
  icon: '📚',
  condition: ({
    kbStats
  }) => (kbStats?.total_entries ?? 0) >= 50,
  progress: ({
    kbStats
  }) => ({
    current: kbStats?.total_entries ?? 0,
    target: 50
  })
}, {
  id: 'spirit_wealth',
  name: t("game.types.k35"),
  description: t("game.types.k36"),
  icon: '💰',
  condition: ({
    world
  }) => world.character.spirit_stones >= 500,
  progress: ({
    world
  }) => ({
    current: Math.min(world.character.spirit_stones, 500),
    target: 500
  })
}, {
  id: 'full_map',
  name: t("game.types.k37"),
  description: t("game.types.k38"),
  icon: '🗺️',
  condition: ({
    world
  }) => {
    let count = 0;
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) count++;
    }));
    return count >= 10;
  },
  progress: ({
    world
  }) => {
    let count = 0;
    world.grid.forEach(row => row.forEach(t => {
      if (t.building) count++;
    }));
    return {
      current: count,
      target: 10
    };
  }
}, {
  id: 'immortal',
  name: t("game.types.k39"),
  description: t("game.types.k40"),
  icon: '⚡',
  condition: ({
    world
  }) => stageOrder(world.character.cultivation_stage) >= 8,
  progress: ({
    world
  }) => ({
    current: stageOrder(world.character.cultivation_stage),
    target: 8
  })
}];
export const BUILDING_EMOJI: Record<string, string> = {
  cottage: '🏠',
  spirit_field: '🌱',
  alchemy_room: '⚗️',
  tower: '🗼',
  city: '🏯',
  academy: '📚',
  palace: '🏰',
  immortal_abode: '🏔️'
};
export const TERRAIN_COLORS: Record<string, string> = {
  plain: '#1a2e1a',
  water: '#0a1a30',
  hill: '#2e2818',
  mountain: '#252525'
};
export const TERRAIN_NAMES: Record<string, string> = {
  plain: t("game.types.k41"),
  water: t("game.types.k42"),
  hill: t("game.types.k43"),
  mountain: t("game.types.k44")
};
export const STAGE_NAMES: Record<string, string> = {
  mortal: t("game.components.RealmCard.k1"),
  qi_refining: t("game.components.RealmCard.k2"),
  foundation_building: t("game.components.RealmCard.k3"),
  golden_core: t("game.components.RealmCard.k4"),
  nascent_soul: t("game.components.RealmCard.k5"),
  spirit_transformation: t("game.components.RealmCard.k6"),
  unity: t("game.components.RealmCard.k7"),
  mahayana: t("game.components.RealmCard.k8"),
  tribulation: t("game.components.RealmCard.k9")
};
export const BUILDING_NAMES: Record<string, string> = {
  cottage: t("game.types.k45"),
  spirit_field: t("game.types.k46"),
  alchemy_room: t("game.types.k47"),
  tower: t("game.types.k48"),
  city: t("game.components.BuildingStatsCard.k3"),
  academy: t("game.types.k49"),
  palace: t("game.components.BuildingStatsCard.k5"),
  immortal_abode: t("game.components.BuildingStatsCard.k8")
};
export const STAGE_COLORS: Record<string, string> = {
  mortal: '#888888',
  qi_refining: '#00F0FF',
  foundation_building: '#00cc44',
  golden_core: '#FFD700',
  nascent_soul: '#B026FF',
  spirit_transformation: '#FF006E',
  unity: '#FF8800',
  mahayana: '#00BFFF',
  tribulation: '#FF4444'
};
export function stageOrder(stage: string): number {
  const order: Record<string, number> = {
    mortal: 0,
    qi_refining: 1,
    foundation_building: 2,
    golden_core: 3,
    nascent_soul: 4,
    spirit_transformation: 5,
    unity: 6,
    mahayana: 7,
    tribulation: 8
  };
  return order[stage] ?? 0;
}
export const XP_THRESHOLDS = [{
  stage: 'mortal',
  xp: 0
}, {
  stage: 'qi_refining',
  xp: 100
}, {
  stage: 'foundation_building',
  xp: 500
}, {
  stage: 'golden_core',
  xp: 1500
}, {
  stage: 'nascent_soul',
  xp: 5000
}, {
  stage: 'spirit_transformation',
  xp: 15000
}, {
  stage: 'unity',
  xp: 50000
}, {
  stage: 'mahayana',
  xp: 150000
}, {
  stage: 'tribulation',
  xp: 500000
}];
export function stageFromXp(totalXp: number): string {
  for (let i = XP_THRESHOLDS.length - 1; i >= 0; i--) {
    if (totalXp >= XP_THRESHOLDS[i].xp) return XP_THRESHOLDS[i].stage;
  }
  return 'mortal';
}
export function calcProgress(totalXp: number): number {
  const stage = stageFromXp(totalXp);
  const idx = XP_THRESHOLDS.findIndex(s => s.stage === stage);
  if (idx >= XP_THRESHOLDS.length - 1) return 1;
  const current = XP_THRESHOLDS[idx].xp;
  const next = XP_THRESHOLDS[idx + 1].xp;
  return Math.min(1, (totalXp - current) / (next - current));
}
export function isBuildableTerrain(terrain: string): boolean {
  return terrain === 'plain' || terrain === 'hill';
}