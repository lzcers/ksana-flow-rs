import React, { useEffect, useMemo, useState } from "react";
import { LoaderCircle, Orbit, Sparkles } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { useGameUIStore } from "../store/gameUIStore";
import type { StartupStage } from "../store/gameUIStore";
import {
  PrimaryButton,
  PageTitle,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StatusPill,
  StoryFrame,
} from "../components/AkashicUI";
import { useGameInternalStore } from "../store/gameStore";

type StepStatus = "pending" | "active" | "done";

interface StartupStep {
  key: ProfileStepKey;
  label: string;
  title: string;
  description: string;
}

interface StageHeadline {
  title: string;
  subtitle: string;
}

type ProfileStepKey =
  | "generating_world"
  | "generating_protagonist"
  | "creating_session";

const startupSteps: StartupStep[] = [
  {
    key: "generating_world",
    label: "世界生成中",
    title: "编织世界脉络",
    description:
      "正在收束时代纹理、核心矛盾与规则压力，让回响拥有可持续展开的张力。",
  },
  {
    key: "generating_protagonist",
    label: "人物生成中",
    title: "凝聚主角命运",
    description:
      "正在把你的烙印、欲望、弱点与性格倾向熔进人物设定，让主角更适合长期演绎。",
  },
  {
    key: "creating_session",
    label: "注入回响中",
    title: "唤起第一轮命运",
    description: "世界与主角设定已经落笔，正在将它们汇入故事，并唤起开场叙事。",
  },
];

const stageOrder: Exclude<StartupStage, "idle">[] = [
  "generating_world",
  "generating_protagonist",
  "creating_session",
  "ready_to_enter",
];

const profileStepOrder: ProfileStepKey[] = [
  "generating_world",
  "generating_protagonist",
  "creating_session",
];

const rotatingMessages: Record<Exclude<StartupStage, "idle">, string[]> = {
  generating_world: [
    "正在编织时代压力",
    "正在收束世界规则与禁忌",
    "正在校准冲突将如何逼近主角",
  ],
  generating_protagonist: [
    "正在收束主角欲望",
    "正在打磨人物弱点与裂缝",
    "正在让性格倾向落成可演绎的行动方式",
  ],
  ready_to_enter: [
    "第一轮叙事已经开始显影",
    "命运入口已经被推开一道缝隙",
    "你现在可以步入回响，直接看到故事继续流动",
  ],
  creating_session: [
    "正在注入世界设定",
    "正在唤起第一轮命运",
    "正在为你铺开故事的开场",
  ],
};

function stepStatus(
  currentStage: StartupStage,
  targetStage: Exclude<StartupStage, "idle">,
): StepStatus {
  const currentIndex = stageOrder.indexOf(
    currentStage === "idle" ? "generating_world" : currentStage,
  );
  const targetIndex = stageOrder.indexOf(targetStage);
  if (targetIndex < currentIndex) {
    return "done";
  }
  if (targetIndex === currentIndex) {
    return "active";
  }
  return "pending";
}

function stageHeadline(
  stage: StartupStage,
  name: string,
  hasPlayableSession: boolean,
): StageHeadline {
  switch (stage) {
    case "generating_protagonist":
      return {
        title: "主角轮廓正在浮现",
        subtitle: `${name} 的欲望、弱点与行动倾向正在被收束成更适合展开剧情的命运底稿。`,
      };
    case "creating_session":
      return {
        title: "回响入口正在开启",
        subtitle:
          "世界设定与主角设定已经生成，正在将它们汇入回响，并点亮第一轮叙事。",
      };
    case "ready_to_enter":
      return hasPlayableSession
        ? {
            title: "开场已经点亮",
            subtitle:
              "第一轮叙事已经开始流动。你现在就可以步入回响，直接看着它继续展开。",
          }
        : {
            title: "入口稍有震颤",
            subtitle:
              "世界设定与主角设定已经完备。再试一次，就能继续唤起你的第一轮命运。",
          };
    case "generating_world":
    case "idle":
    default:
      return {
        title: "世界纹理正在编织",
        subtitle:
          "回响会先推演世界压力，再收束主角命运，让开场更像一个真正会继续生长的故事。",
      };
  }
}

function profileStepFromStage(stage: Exclude<StartupStage, "idle">): ProfileStepKey {
  return stage === "generating_world" || stage === "generating_protagonist"
    ? stage
    : "creating_session";
}

const GeneratingPage: React.FC = () => {
  const {
    startupStage,
    character,
    world,
    preparedProfiles,
    isLoading,
    error,
    startGame,
    enterWorld,
    hasPlayableSession,
  } = useGameUIStore(
    useShallow((state) => ({
      startupStage: state.startupStage,
      character: state.character,
      world: state.world,
      preparedProfiles: state.preparedProfiles,
      isLoading: state.isLoading,
      error: state.error,
      startGame: state.startGame,
      enterWorld: state.enterWorld,
      hasPlayableSession: Boolean(state.stateView),
    })),
  );
  const sessionId = useGameInternalStore((state) => state.sessionId);
  const canEnterWorld =
    startupStage === "ready_to_enter" &&
    Boolean(sessionId) &&
    hasPlayableSession;
  const headline = stageHeadline(
    startupStage,
    character.name || "这位主角",
    canEnterWorld,
  );
  const stageKey = startupStage === "idle" ? "generating_world" : startupStage;
  const currentMessages = useMemo(() => {
    if (stageKey !== "ready_to_enter") {
      return rotatingMessages[stageKey];
    }

    return canEnterWorld
      ? [
          "第一轮叙事已经开始显影",
          "命运入口已经被推开一道缝隙",
          "你现在可以步入回响，直接看到故事继续流动",
        ]
      : [
          "设定已经落笔，只差把旅程重新续上",
          "命运入口短暂摇晃，你可以再次尝试",
          "再推开一次门，回响会继续向前",
        ];
  }, [canEnterWorld, stageKey]);
  const messageKey = currentMessages.join("||");
  const [messageCursor, setMessageCursor] = useState({ key: "", index: 0 });
  const messageIndex = messageCursor.key === messageKey
    ? Math.min(messageCursor.index, Math.max(currentMessages.length - 1, 0))
    : 0;
  const [selectedProfileOverride, setSelectedProfileOverride] = useState<{
    stageKey: Exclude<StartupStage, "idle">;
    key: ProfileStepKey;
  } | null>(null);
  const selectedProfileStep = selectedProfileOverride?.stageKey === stageKey
    ? selectedProfileOverride.key
    : profileStepFromStage(stageKey);
  const profilePanels = useMemo(() => {
    if (!preparedProfiles) {
      return null;
    }

    return {
      generating_world: {
        eyebrow: "世界设定",
        title: "这一次回响所落下的世界纹理",
        text: preparedProfiles.world,
        className: "border-[#5b6f96]/30 bg-[#0f1624]/80 text-[#c7d5f2]",
      },
      generating_protagonist: {
        eyebrow: "人物设定",
        title: "主角被命运收束后的轮廓",
        text: preparedProfiles.protagonist,
        className: "border-[#6f5f96]/30 bg-[#151325]/80 text-[#d8d0f2]",
      },
      creating_session: {
        eyebrow: "命运引线",
        title: "故事即将被点亮的第一束牵引",
        text: preparedProfiles.keyStoryBeats,
        className: "border-[#8a7755]/30 bg-[#17120f]/80 text-[#efe4cd]/88",
      },
    } satisfies Record<
      ProfileStepKey,
      {
        eyebrow: string;
        title: string;
        text: string;
        className: string;
      }
    >;
  }, [preparedProfiles]);

  useEffect(() => {
    if (currentMessages.length <= 1) {
      return undefined;
    }

    const timer = window.setInterval(() => {
      setMessageCursor((current) => {
        const currentIndex = current.key === messageKey ? current.index : 0;
        return {
          key: messageKey,
          index: (currentIndex + 1) % currentMessages.length,
        };
      });
    }, 2200);

    return () => window.clearInterval(timer);
  }, [currentMessages.length, messageKey]);

  const activeProfilePanel = profilePanels?.[selectedProfileStep];

  return (
    <ScreenShell className="items-center">
      <StoryFrame className="max-w-3xl p-4 md:p-6">
        <div className="space-y-4 md:space-y-6">
          <PageTitle title={headline.title} subtitle={headline.subtitle} />

          <div className="rounded-[1.1rem] border border-[#6d86b7]/25 bg-[#101827]/78 px-4 py-3 text-center text-sm text-[#c7d5f2] shadow-[0_10px_30px_rgba(3,8,18,0.25)]">
            {currentMessages[messageIndex]}
          </div>

          {error ? (
            <div className="rounded-[1.1rem] border border-[#7f3b3b]/50 bg-[#2a1216]/85 px-4 py-3 text-sm text-[#ffd7d7]">
              {error}
            </div>
          ) : null}

          <SectionCard className="space-y-4">
            <div className="flex flex-wrap gap-2">
              <StatusPill
                icon={Orbit}
                className="border-[#3b82f6]/30 bg-[#0f2141]/80 text-[#cfe0ff]"
              >
                {world.era}
              </StatusPill>
              <StatusPill
                icon={Sparkles}
                className="border-[#8b5cf6]/30 bg-[#1b1733]/80 text-[#e3d8ff]"
              >
                {character.background || "命运烙印待揭晓"}
              </StatusPill>
            </div>

            <div className="space-y-3">
              {startupSteps.map((step) => {
                const status = stepStatus(startupStage, step.key);
                const isSelected = profilePanels
                  ? selectedProfileStep === step.key
                  : false;
                const iconClassName =
                  status === "active"
                    ? "text-[#7dd3fc] animate-spin"
                    : status === "done"
                      ? "text-[#f4d58d]"
                      : "text-[#5f6c86]";

                return (
                  <button
                    type="button"
                    key={step.key}
                      onClick={
                        profilePanels
                          ? () => setSelectedProfileOverride({ stageKey, key: step.key })
                          : undefined
                      }
                    className={`rounded-[1.1rem] border px-4 py-4 transition-colors md:px-5 ${
                      status === "active"
                        ? "border-[#60a5fa]/40 bg-[#101a2c]/92"
                        : status === "done"
                          ? "border-[#8a7755]/35 bg-[#14110f]/85"
                          : "border-white/8 bg-[#0f1420]/70"
                    } ${profilePanels ? "w-full cursor-pointer text-left hover:border-[#8fa4ca]/45" : "w-full text-left"} ${isSelected ? "ring-1 ring-[#c7d5f2]/45" : ""}`}
                  >
                    <div className="flex items-start gap-3">
                      <LoaderCircle
                        className={`mt-0.5 h-5 w-5 shrink-0 ${iconClassName}`}
                      />
                      <div className="space-y-1">
                        <p className="text-sm font-semibold tracking-wide text-[#efe4cd]">
                          {step.label}
                        </p>
                        <p className="text-base font-medium text-[#f8f1e3] md:text-lg">
                          {step.title}
                        </p>
                        <p className="text-sm leading-6 text-[#9ca7be]">
                          {step.description}
                        </p>
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </SectionCard>

          {activeProfilePanel ? (
            <SectionCard className="space-y-4">
              <div className="flex items-start justify-between gap-4">
                <p className="text-xs font-medium tracking-[0.24em] text-[#8fa4ca]"></p>
                <div className="flex gap-2">
                  {profileStepOrder.map((key) => (
                    <button
                      key={key}
                      type="button"
                      aria-label={`切换到${profilePanels[key].eyebrow}`}
                      onClick={() => setSelectedProfileOverride({ stageKey, key })}
                      className={`h-2.5 w-2.5 rounded-full transition-colors ${selectedProfileStep === key ? "bg-[#d8c58a]" : "bg-white/18 hover:bg-white/35"}`}
                    />
                  ))}
                </div>
              </div>

              <div
                className={`rounded-[1.1rem] border p-4 md:p-5 ${activeProfilePanel.className}`}
              >
                <p className="text-sm font-semibold tracking-wide text-[#efe4cd]">
                  {activeProfilePanel.eyebrow}
                </p>
                <p className="mt-2 text-base font-medium text-[#f8f1e3] md:text-lg">
                  {activeProfilePanel.title}
                </p>
                <p className="mt-4 whitespace-pre-wrap text-sm leading-7">
                  {activeProfilePanel.text}
                </p>
              </div>
            </SectionCard>
          ) : null}

          {preparedProfiles ? (
            <div className="flex flex-col items-center justify-center gap-3 sm:flex-row">
              <SecondaryButton
                onClick={() => void startGame()}
                className="min-w-44"
              >
                重新编织
              </SecondaryButton>
              <PrimaryButton
                onClick={() => void enterWorld()}
                disabled={!canEnterWorld && isLoading}
                className="min-w-44"
              >
                {canEnterWorld
                  ? "步入回响"
                  : isLoading
                    ? "回响共鸣中..."
                    : "再次步入回响"}
              </PrimaryButton>
            </div>
          ) : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default GeneratingPage;
