import { memo, useState, useEffect, useRef } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Maximize2, Clapperboard } from 'lucide-react';
import { parse } from 'jsonriver';
import { NodeWrapper } from './NodeWrapper';
import { FullScreenModal } from '../ui/FullScreenModal';
import { ShortVideoCreation } from '../ShortVideoCreation';
import type { ProjectData } from '../ShortVideoCreation/types';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import './index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left];
const testdata = {
  "script": {
    "content": "【开场】\n航天中心走廊，我疲惫地走着。主任叫住我，递来一副奇特的眼镜。\n\n主任：休假可以，带上这个。\n我：传感眼镜？给谁用？\n主任：一位领航员。她在太空执行长期任务，需要感受地球。\n\n【草原篇】\n我戴上眼镜，眼前浮现HUD界面。一个年轻女声响起。\n\n她：哇...这就是草原吗？\n我：改造后的塔克拉玛干。\n\n镜头切换：我的视角与眼镜共享视角并置。\n她让我触摸野花，感受露水，聆听风声。她对每处细节都发出惊叹。\n\n她：你能闻到青草香吗？\n我：...能。\n她：谢谢你。\n\n夜晚，我困倦地靠在帐篷外。眼镜里传来她的声音。\n\n她：请不要关掉眼镜...我想看月亮。\n我：你那边看不到月亮？\n她：（停顿）...看不到。\n\n【疑点浮现】\n我注意到异常：通讯毫无延时。她的太空服隔热层异常厚重。一次她差点说“这里好热”，又急忙改口。\n\n【转折】\n回到航天中心，我查阅资料。屏幕上显示“落日工程-地心探测”。\n我猛地站起，冲向主任办公室。\n\n我：她根本不在太空！她在哪里？！\n主任沉默地调出监控画面：一个暗红色的封闭船舱，失重的铅笔漂浮着。\n\n主任：落日六号，三年前失事在地核交界层。她是唯一幸存者。\n\n【真相】\n我重新戴上眼镜。画面分割：左侧是草原阳光，右侧是地心船舱的冷光。\n\n她：你知道了？\n我：...为什么不说？\n她：说了又能改变什么呢？至少...我有了这片草原。\n\n她开始描述“看”到的草原：每一朵花，每一缕光，每一滴雨。\n\n她：我的世界很小，但你们的很大。请替我好好看看它。\n\n【尾声】\n眼镜能量警报响起。\n\n她：时间到了。\n我：等等——\n她：再见。谢谢你，我的草原。\n\n连接中断。\n\n我低头，裤脚上粘着一颗草籽。\n\n窗外，真正的草原在阳光下延伸。\n\n【字幕】她的眼睛，成了我们的星空。"
  },
  "characters": [
    {
      "id": "char_01",
      "name": "我",
      "avatar": "",
      "description": "航天系统工程师，30余岁。长期高压工作导致情感麻木，对世界失去感知力。专业敏锐，能发现技术细节中的异常。经历事件后逐渐被唤醒。",
      "tags": [
        "工程师",
        "情感麻木",
        "观察者",
        "被改变者"
      ]
    },
    {
      "id": "char_02",
      "name": "她",
      "avatar": "",
      "description": "“落日六号”地航飞船领航员，20多岁。被困地心深处，是飞船失事后的唯一幸存者。身处绝境却保持着惊人的感知力与精神强度，对自然之美抱有极致的热爱。通过传感眼镜与地表世界保持最后连接。",
      "tags": [
        "领航员",
        "被困者",
        "感知敏锐",
        "坚韧",
        "悲剧性"
      ]
    },
    {
      "id": "char_03",
      "name": "主任",
      "avatar": "",
      "description": "航天中心负责人，50岁左右。知晓“落日六号”事故真相，背负沉重秘密。态度严肃，在真相揭露时刻展现出无奈与沉重。",
      "tags": [
        "负责人",
        "知情者",
        "沉重",
        "关键信息提供者"
      ]
    }
  ],
  "storyboard": [
    {
      "id": "shot_01",
      "shotNo": 1,
      "image": "",
      "description": {
        "background": "航天中心现代感走廊，冷色调灯光，金属质感墙面。",
        "relation": "主任叫住正低头走路的‘我’。",
        "composition": "中景，主任从画面一侧入画，手搭在‘我’肩上。"
      },
      "lines": {
        "narration": "",
        "dialogue": "主任：休假可以，带上这个。\n我：传感眼镜？给谁用？\n主任：一位领航员。她在太空执行长期任务，需要感受地球。"
      },
      "mainCharacter": "char_01",
      "shotSize": "中景",
      "cameraAngle": "视平",
      "lensType": "双人镜头",
      "duration": 8
    },
    {
      "id": "shot_02",
      "shotNo": 2,
      "image": "",
      "description": {
        "background": "改造后的塔克拉玛干草原，绿草如茵，野花点缀，远处有防风林。",
        "relation": "‘我’独自站在草原上，戴着传感眼镜。",
        "composition": "近景，侧拍‘我’的侧脸，眼镜边缘有微光。"
      },
      "lines": {
        "narration": "",
        "dialogue": "她（OS，兴奋地）：哇...这就是草原吗？\n我：改造后的塔克拉玛干。"
      },
      "mainCharacter": "char_01",
      "shotSize": "近景",
      "cameraAngle": "视平",
      "lensType": "单人镜头",
      "duration": 5
    },
    {
      "id": "shot_03",
      "shotNo": 3,
      "image": "",
      "description": {
        "background": "分屏画面。左侧：草原上的一朵小野花特写。右侧：暗红色调的封闭船舱内，冷光仪器面板。",
        "relation": "通过技术手段并置两个世界。",
        "composition": "特写野花，画面逐渐分割，右侧浮现地心船舱画面。"
      },
      "lines": {
        "narration": "",
        "dialogue": "她（OS）：你能...摸一下那朵花吗？\n（‘我’的手入画，轻触花瓣）\n她（OS，带着颤音）：...谢谢。"
      },
      "mainCharacter": "char_02",
      "shotSize": "特写",
      "cameraAngle": "俯视（对花）/ 视平（对船舱）",
      "lensType": "主观镜头/客观镜头并置",
      "duration": 7
    },
    {
      "id": "shot_04",
      "shotNo": 4,
      "image": "",
      "description": {
        "background": "草原夜晚，帐篷外，星空低垂，月光洒落。",
        "relation": "‘我’靠在帐篷外打盹，眼镜仍闪着微光。",
        "composition": "中景，‘我’半躺在折叠椅上，背景是星空和帐篷轮廓。"
      },
      "lines": {
        "narration": "",
        "dialogue": "她（OS，轻声）：请不要关掉眼镜...我想看月亮。\n我（迷糊地）：你那边看不到月亮？\n她（OS，停顿）：...看不到。"
      },
      "mainCharacter": "char_01",
      "shotSize": "中景",
      "cameraAngle": "微俯",
      "lensType": "单人镜头",
      "duration": 8
    },
    {
      "id": "shot_05",
      "shotNo": 5,
      "image": "",
      "description": {
        "background": "航天中心资料室，多个屏幕亮着。",
        "relation": "‘我’快速翻阅资料，表情越来越凝重。",
        "composition": "近景，屏幕反光在‘我’脸上闪烁，显示‘落日工程’‘地心探测’‘古腾堡不连续面’等关键词。"
      },
      "lines": {
        "narration": "我发现了矛盾。无延时的通讯。异常厚重的隔热服。她无意中流露的对‘热’的恐惧。",
        "dialogue": ""
      },
      "mainCharacter": "char_01",
      "shotSize": "近景",
      "cameraAngle": "视平",
      "lensType": "单人镜头",
      "duration": 6
    },
    {
      "id": "shot_06",
      "shotNo": 6,
      "image": "",
      "description": {
        "background": "主任办公室，简洁严肃。",
        "relation": "‘我’冲进办公室质问，主任沉默地操作控制台。",
        "composition": "双人镜头，‘我’在前景激动，主任在背景操作屏幕。"
      },
      "lines": {
        "narration": "",
        "dialogue": "我：她根本不在太空！她在哪里？！\n（主任没有回答，调出一个监控画面）"
      },
      "mainCharacter": "char_01",
      "shotSize": "中景",
      "cameraAngle": "视平",
      "lensType": "双人镜头",
      "duration": 5
    },
    {
      "id": "shot_07",
      "shotNo": 7,
      "image": "",
      "description": {
        "background": "主任的屏幕特写：显示一个暗红、橙色调的封闭船舱内部。一支铅笔失重地漂浮着。仪器发出冷光。",
        "relation": "真相揭露。",
        "composition": "特写屏幕画面，强调失重的铅笔和压抑的环境。"
      },
      "lines": {
        "narration": "",
        "dialogue": "主任（OS，沉重地）：落日六号，三年前失事在地核交界层。她是唯一幸存者。"
      },
      "mainCharacter": "char_03",
      "shotSize": "特写",
      "cameraAngle": "视平",
      "lensType": "客观镜头",
      "duration": 6
    },
    {
      "id": "shot_08",
      "shotNo": 8,
      "image": "",
      "description": {
        "background": "分屏画面强化。左侧：阳光下的草原，色彩饱满。右侧：地心船舱，暗红单调。",
        "relation": "‘我’重新连接，双方都知道了真相。",
        "composition": "对半分割画面，形成视觉与情感上的绝对对比。"
      },
      "lines": {
        "narration": "",
        "dialogue": "她（OS，平静地）：你知道了？\n我（OS）：...为什么不说？\n她（OS）：说了又能改变什么呢？至少...我有了这片草原。"
      },
      "mainCharacter": "char_02",
      "shotSize": "特写（分屏）",
      "cameraAngle": "视平",
      "lensType": "主观镜头并置",
      "duration": 10
    },
    {
      "id": "shot_09",
      "shotNo": 9,
      "image": "",
      "description": {
        "background": "镜头快速切换：野花、溪流、阳光、风中的草浪、雨滴、月光...都是之前草原之旅的片段。",
        "relation": "她描述着她‘看’到和记住的世界。",
        "composition": "快速蒙太奇，每个镜头都美丽而充满生命力。"
      },
      "lines": {
        "narration": "",
        "dialogue": "她（OS，充满感情）：我的世界很小，但你们的很大。请替我好好看看它。"
      },
      "mainCharacter": "char_02",
      "shotSize": "多种（特写、近景、全景）",
      "cameraAngle": "多种",
      "lensType": "蒙太奇序列",
      "duration": 8
    },
    {
      "id": "shot_10",
      "shotNo": 10,
      "image": "",
      "description": {
        "background": "‘我’的视角，眼镜的HUD界面出现红色能量警报，闪烁。",
        "relation": "连接即将中断。",
        "composition": "第一人称主观镜头，聚焦于闪烁的警报图标。"
      },
      "lines": {
        "narration": "",
        "dialogue": "（警报声）\n她（OS，轻柔但坚定）：时间到了。\n我（OS）：等等——\n她（OS）：再见。谢谢你，我的草原。\n（连接切断的电流声）"
      },
      "mainCharacter": "char_01",
      "shotSize": "特写",
      "cameraAngle": "主观",
      "lensType": "主观镜头",
      "duration": 7
    },
    {
      "id": "shot_11",
      "shotNo": 11,
      "image": "",
      "description": {
        "background": "航天中心房间内，窗外可见城市或远山。",
        "relation": "‘我’低头看着自己的裤脚。",
        "composition": "特写，裤脚上粘着一颗小小的、干枯的草籽。"
      },
      "lines": {
        "narration": "",
        "dialogue": ""
      },
      "mainCharacter": "char_01",
      "shotSize": "特写",
      "cameraAngle": "俯视",
      "lensType": "客观镜头",
      "duration": 4
    },
    {
      "id": "shot_12",
      "shotNo": 12,
      "image": "",
      "description": {
        "background": "真实的、广阔的草原，阳光灿烂，风吹草浪，无边无际。",
        "relation": "‘我’站在窗前，或只是一个空镜头，展现世界的辽阔。",
        "composition": "大远景/全景，强调草原的广阔与生命力。"
      },
      "lines": {
        "narration": "她的眼睛，成了我们的星空。",
        "dialogue": ""
      },
      "mainCharacter": "",
      "shotSize": "全景/大远景",
      "cameraAngle": "视平",
      "lensType": "客观镜头",
      "duration": 5
    }
  ]
}
export const ShortVideoNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [projectData, setProjectData] = useState<ProjectData | null>(testdata || null);


  const lastMessage = data.lastMessage;
  const streamControllerRef = useRef<ReadableStreamDefaultController<string> | null>(null);
  const processedLengthRef = useRef(0);

  useEffect(() => {
    if (!lastMessage) {
      if (streamControllerRef.current) {
        try { streamControllerRef.current.close(); } catch { }
        streamControllerRef.current = null;
      }
      processedLengthRef.current = 0;
      return;
    }

    if (lastMessage.length < processedLengthRef.current) {
      if (streamControllerRef.current) {
        try { streamControllerRef.current.close(); } catch { }
        streamControllerRef.current = null;
      }
      processedLengthRef.current = 0;
    }

    if (!streamControllerRef.current) {
      const stream = new ReadableStream<string>({
        start(controller) {
          streamControllerRef.current = controller;
        }
      });

      (async () => {
        try {
          const parser = parse(stream as unknown as AsyncIterable<string>);
          for await (const value of parser) {
            console.log(value)
            if (value && typeof value === 'object') {
              setProjectData(value as unknown as ProjectData);
            }
          }
        } catch (e) {
          console.debug('JSON stream parsing ended', e);
        }
      })();
    }

    // Enqueue new content
    const newContent = lastMessage.slice(processedLengthRef.current);
    if (newContent.length > 0) {
      streamControllerRef.current?.enqueue(newContent);
      processedLengthRef.current = lastMessage.length;
    }
  }, [lastMessage]);

  const headerActions = (
    <div className="flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="Open Editor"
      >
        <Maximize2 size={12} />
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={240}
      minHeight={200}
      style={{ width: width ?? 300, height: height ?? 300 }}
      headerActions={headerActions}
    >
      <div className="p-4 flex-1 flex flex-col items-center justify-center min-h-0 bg-zinc-950/50">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center mx-auto shadow-lg shadow-indigo-500/20">
            <Clapperboard className="text-white" size={24} />
          </div>
          <div>
            <h3 className="text-sm font-medium text-zinc-200">AI Short Video Creator</h3>
            <p className="text-xs text-zinc-500 mt-1">
              {projectData ? 'Project ready' : 'Waiting for input...'}
            </p>
          </div>

          <button
            onClick={() => setIsFullScreen(true)}
            className="px-4 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs rounded-md transition-colors border border-zinc-700"
          >
            Open Studio
          </button>
        </div>

        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={() => setIsFullScreen(false)}
            title="Short Video Studio"
            className="bg-white dark:bg-zinc-950"
          >
            <ShortVideoCreation
              data={projectData as ProjectData}
              onBack={() => setIsFullScreen(false)}
              onDataChange={(newData) => {
                setProjectData(newData);
                updateNodeData(id, {
                  config: { ...data.config, projectData: newData }
                });
              }}
            />
          </FullScreenModal>
        )}
      </div>
    </NodeWrapper>
  );
};

export const ShortVideoNode = memo(ShortVideoNodeComponent);
