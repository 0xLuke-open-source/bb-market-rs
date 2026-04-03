const PORTAL_FALLBACK_PULSE = [
  { symbol: 'BTCUSDT', score: 92, tag: '强势拉升', change: '+3.82%' },
  { symbol: 'ETHUSDT', score: 86, tag: '资金回流', change: '+2.47%' },
  { symbol: 'SOLUSDT', score: 81, tag: '鲸鱼活跃', change: '+5.13%' },
  { symbol: 'BNBUSDT', score: 74, tag: '盘口修复', change: '+1.36%' },
  { symbol: 'DOGEUSDT', score: 69, tag: '短线放量', change: '+4.28%' }
];

const PORTAL_FOOTER_GROUPS = [
  {
    title: '信息与支持',
    links: [
      { page: 'help', label: '帮助中心' },
      { page: 'feedback', label: '产品反馈与建议' },
      { page: 'announcements', label: '公告' },
      { page: 'news', label: '新闻中心' }
    ]
  },
  {
    title: '内容与社区',
    links: [
      { page: 'plaza', label: '广场' },
      { page: 'blog', label: '博客' },
      { page: 'community', label: '社区' },
      { page: 'about', label: '关于我们' }
    ]
  },
  {
    title: '平台说明',
    links: [
      { page: 'vip', label: '机构和VIP服务' },
      { page: 'agreement', label: '服务协议' },
      { page: 'privacy', label: '隐私说明' }
    ]
  }
];

const PORTAL_PAGES = {
  ai: {
    kicker: 'AI Radar',
    title: 'AI盯盘中枢',
    lead: '把盘口、成交、鲸鱼、异常、主动买卖量差和策略信号收敛到一个统一的判断面板，给交易员提供秒级决策入口。',
    highlights: ['毫秒级盘口扫描', '多因子评分引擎', '公开预览与订阅解锁', '强提醒与异常回放'],
    metrics: runtime => [
      { label: '在线监控币对', value: runtime.totalSymbols, note: '来自当前服务状态' },
      { label: '强提醒候选', value: runtime.strongSignals, note: '泵盘 / 砸盘综合评分 >= 70' },
      { label: '鲸鱼异动', value: runtime.whales, note: '大额挂单与主动吃单事件' },
      { label: '最近告警流', value: runtime.feedCount, note: '本次会话已接收 feed / alert 条目' }
    ],
    sections: [
      {
        type: 'cards',
        title: 'AI能力模块',
        desc: '前端页面给用户看的不是技术名词堆砌，而是能直接转化为动作的信号。',
        items: [
          { title: '盘口失衡引擎', body: '结合 OBI、OFI、买卖墙密度、价差和深度倾斜度，识别“看起来强”和“真正强”的区别。', meta: '盘口层' },
          { title: '主动买卖量差引擎', body: '用 CVD 与 taker buy ratio 拆分情绪驱动与跟风成交，过滤低质量放量。', meta: '成交层' },
          { title: '鲸鱼轨迹识别', body: '追踪大额挂单、撤单、吃单与重复性扫单，把“有钱人动作”从噪声里拎出来。', meta: '大户层' },
          { title: '异常结构检测', body: '针对短时剧烈偏移、订单簿断层、刷量痕迹和伪突破给出高优先级提示。', meta: '风控层' }
        ]
      },
      {
        type: 'list',
        title: '典型使用路径',
        desc: '适合交易员、主播、社群运营和策略研究员的统一操作面。',
        items: [
          { title: '盘前筛选', text: '先看综合优先 + 鲸鱼区块，快速锁定今天最可能出波动的币种。' },
          { title: '盘中盯防', text: '用实时信号墙盯住强提醒、异常和主动买卖量差扭转，减少纯主观盯盘。' },
          { title: '盘后复盘', text: '通过回放接口复现关键时刻的订单、余额、成交与事件，沉淀策略模板。' }
        ]
      },
      {
        type: 'faq',
        title: 'AI盯盘 FAQ',
        desc: '把用户最常问的几件事直接放在页面里，减少咨询成本。',
        items: [
          { q: 'AI盯盘是不是自动下单？', a: '不是。默认是信号和分析中枢，是否下单仍然由交易员确认；你也可以在现有交易面板中手动执行。' },
          { q: '公开模式能看到什么？', a: '公开模式展示部分币种与基础实时流，登录后可以绑定账户，订阅后解锁完整币种池、完整推送和更深层功能。' },
          { q: '是否支持自定义策略？', a: '当前页面以平台内置逻辑为主，后续可以在“产品反馈与建议”页提交你需要的自定义因子。' }
        ]
      }
    ]
  },
  vip: {
    kicker: 'VIP Service',
    title: '机构和VIP服务',
    lead: '面向交易席位、量化团队与机构桌面，提供分层权限、席位协同、专属支持与定制交付的一体化终端服务。',
    highlights: ['席位分层授权', '专属支持体系', '策略共研', '终端化交付'],
    metrics: runtime => [
      { label: '开放套餐', value: runtime.planCount, note: '当前接口返回可订阅计划数' },
      { label: '机构客户线索', value: '128', note: '近 30 日有效咨询' },
      { label: '续费率', value: '78.4%', note: '季度服务续费' },
      { label: '平均响应', value: '7 分钟', note: '专属支持工作时段' }
    ],
    sections: [
      {
        type: 'cards',
        title: '服务层级',
        desc: '按监控深度、协作规模与交付方式划分，便于团队快速判断适配层级。',
        items: [
          { title: '个人 Pro', body: '适合高频盯盘与多标的切换的活跃交易员，重点解锁完整监控池、实时推送与更深层结构分析。', meta: '个人席位' },
          { title: 'Desk 团队版', body: '适合 3-20 人交易或研究小组，支持席位管理、内部协同、权限分层与团队级工作台。', meta: '研究与交易团队' },
          { title: '机构私有版', body: '支持隔离部署、白名单接入、日志保留、独立风控与接口定制，满足机构桌面与合规交付要求。', meta: '机构桌面' }
        ]
      },
      {
        type: 'table',
        title: '权益矩阵',
        desc: '将能力边界、支持方式与部署级别一次说明，减少沟通成本。',
        columns: ['能力', '公开模式', '个人 Pro', 'Desk 团队版', '机构私有版'],
        rows: [
          ['实时币种池', '部分可见', '全量', '全量', '全量 + 定制'],
          ['强提醒推送', '基础', '高级', '高级', '高级 + 私有策略'],
          ['多席位协同', '不支持', '1席位', '3-20席位', '按需配置'],
          ['专属客服', '工单', '基础', '专属群', '专属经理'],
          ['私有部署', '不支持', '不支持', '可选', '标准支持']
        ]
      },
      {
        type: 'list',
        title: '机构常见诉求',
        desc: '围绕不同使用形态概括典型诉求，便于方案判断与交付确认。',
        items: [
          { title: '量化团队', text: '需要稳定的实时信号、统一的复盘链路，以及与内部风控、执行系统配合的接口能力。' },
          { title: '交易席位', text: '需要低切换成本的盘中界面，把信号、订单簿、成交与处置入口放在同一工作流。' },
          { title: '项目方与做市团队', text: '需要持续观察订单簿结构、异常波动与流动性状态，及时识别并处置风险。' }
        ]
      }
    ]
  },
  ads: {
    kicker: 'Ad Network',
    title: '广告解决方案',
    lead: '围绕交易终端、研究内容与高频访问场景设计资源合作方案，帮助合作方明确触达位置、目标人群与效果回收方式。',
    highlights: ['终端曝光位', '专题联动', '研究内容合作', '效果复盘'],
    metrics: () => [
      { label: '月均曝光', value: '240万+', note: '站内页面与社群联动' },
      { label: '平均 CTR', value: '3.8%', note: '核心推荐资源位' },
      { label: '合作品牌', value: '56', note: '近 12 个月已合作项目' },
      { label: '最快上线', value: '24 小时', note: '素材齐备后' }
    ],
    sections: [
      {
        type: 'cards',
        title: '可售资源位',
        desc: '将主要合作位置与适用场景说明清楚，便于合作方直接判断接入方式。',
        items: [
          { title: '首页核心推荐位', body: '适合交易所、数据服务、研究工具与策略品牌，在高意向访问流量中完成稳定触达。', meta: '终端曝光' },
          { title: 'AI 盯盘专题合作', body: '在高关注度盘中场景内配置专题卡、说明页与跳转入口，强化目标用户识别。', meta: '精准触达' },
          { title: '研究内容联合呈现', body: '结合专题解读、方法文章与联名栏目，承接更长期的专业受众曝光。', meta: '内容合作' }
        ]
      },
      {
        type: 'table',
        title: '投放套餐示例',
        desc: '用标准化套餐展示资源组合、合作周期与覆盖水平，后续可直接替换真实报价。',
        columns: ['套餐', '展示周期', '资源位', '预计曝光', '参考预算'],
        rows: [
          ['Starter', '7天', '首页推荐 + 公告联动', '18万', '8,800 USDT'],
          ['Growth', '14天', '首页 + 专题 + 社群转发', '52万', '22,000 USDT'],
          ['Launch', '30天', '全站联动 + 社区活动', '130万', '58,000 USDT']
        ]
      },
      {
        type: 'faq',
        title: '广告合作说明',
        desc: '提前说明审核、素材与数据回传规则，降低合作确认成本。',
        items: [
          { q: '支持哪些素材形式？', a: '支持横幅、卡片、长图、视频封面、落地页跳转和外部活动报名页。' },
          { q: '投放前是否审核项目？', a: '会。涉及高风险收益承诺、违规引流、虚假空投与不合规金融宣传的项目不予合作。' },
          { q: '是否提供数据回传？', a: '可按合作方案提供曝光、点击、跳转、报名与行为转化等核心复盘数据。' }
        ]
      }
    ]
  },
  feedback: {
    kicker: 'Feedback Loop',
    title: '产品反馈与建议',
    lead: '将实盘问题、功能建议与策略需求汇集到统一入口，形成可分类、可回执、可跟踪的终端输入机制。',
    highlights: ['需求收集', '异常回报', '优先级回执', '版本沟通'],
    metrics: () => [
      { label: '本月收集建议', value: '316', note: '含站内与社群反馈' },
      { label: '已采纳', value: '74', note: '进入排期或已发布' },
      { label: 'Bug 修复', value: '41', note: '近30日完成' },
      { label: '平均回执', value: '12 小时', note: '工作日' }
    ],
    sections: [
      {
        type: 'list',
        title: '我们希望收到的反馈',
        desc: '反馈越接近真实盘中场景，进入评估与落地的效率越高。',
        items: [
          { title: '交易工作流痛点', text: '例如在哪个节点最容易漏信号、误判结构、错过执行或无法完成复盘。' },
          { title: '新增因子与数据需求', text: '例如资金费率、新闻事件、链上地址、社媒热度或更细的盘口因子。' },
          { title: '具体终端异常', text: '建议附带页面路径、发生时间、浏览器环境、账户状态与复现步骤，便于快速定位。' }
        ]
      },
      {
        type: 'cards',
        title: '反馈处理流程',
        desc: '明确处理节点与时效，让用户知道反馈如何进入版本与排期。',
        items: [
          { title: '1. 收集与归类', body: '按 Bug、体验优化、新功能与合作需求进入对应处理队列。', meta: 'T+0' },
          { title: '2. 评估优先级', body: '综合影响范围、实现成本、商业价值与风险等级进行排序。', meta: 'T+1' },
          { title: '3. 回执与排期', body: '对重点需求给出是否采纳、预计版本窗口或可行替代方案。', meta: 'T+2' }
        ]
      },
      {
        type: 'faq',
        title: '提交建议前先看',
        desc: '先回答高频问题，减少重复提交。',
        items: [
          { q: '哪里提功能需求最快？', a: '优先通过站内表单或社群管理员提交，附上使用场景、截图与标的示例会更快进入评估。' },
          { q: '怎么确认需求有没有被接收？', a: '页面会显示回执时效，重点需求会收到明确回复，必要时进入公告或版本说明。' },
          { q: '可不可以直接约演示？', a: '可以。涉及机构接入、团队采购或更深的终端评估，建议直接走 VIP 服务页对接。' }
        ]
      }
    ]
  },
  rebate: {
    kicker: 'Rebate Program',
    title: '超级返佣',
    lead: '以清晰的等级规则、结算口径与渠道支持体系承接长期合作，让合作方明确知道如何参与、如何结算与如何稳定放量。',
    highlights: ['等级返佣', '链路透明', '月度结算', '渠道支持'],
    metrics: () => [
      { label: '合作交易所', value: '9', note: '支持返佣跟踪' },
      { label: '最高返佣', value: '55%', note: '视渠道等级而定' },
      { label: '月发放佣金', value: '128,000 USDT', note: '示例结算数据' },
      { label: '活跃推广者', value: '1,460', note: '近30日' }
    ],
    sections: [
      {
        type: 'table',
        title: '返佣等级示例',
        desc: '以阶梯方式说明有效交易额、返佣比例与附加支持。',
        columns: ['等级', '月度有效交易额', '返佣比例', '附加权益'],
        rows: [
          ['R1', '0 - 20万 USDT', '25%', '基础推广链接'],
          ['R2', '20万 - 100万 USDT', '35%', '专属海报与数据看板'],
          ['R3', '100万 - 500万 USDT', '45%', '社群支持 + 活动资源'],
          ['R4', '500万 USDT+', '55%', '专属经理 + 联合品牌位']
        ]
      },
      {
        type: 'cards',
        title: '返佣玩法',
        desc: '不仅说明比例，更要说明适用场景与稳定推进方式。',
        items: [
          { title: '研究内容导流', body: '适合方法文章、盘中解读、直播复盘等内容型推广，通过开户链接与专题页承接转化。', meta: '内容型推广' },
          { title: '社群协同转化', body: '适合社区主理人与招商团队，通过教学活动、实盘分享与群任务提升转化效率。', meta: '社群运营' },
          { title: '渠道合作', body: '适合拥有稳定交易用户资源的团队，可通过后台或接口持续追踪实际贡献。', meta: '渠道合作' }
        ]
      }
    ]
  },
  invite: {
    kicker: 'Referral Growth',
    title: '邀请奖励',
    lead: '围绕注册、首登、订阅转化与榜单激励设计清晰的邀请机制，便于用户理解门槛、路径与奖励效率。',
    highlights: ['注册奖励', '转化奖励', '榜单激励', '活动任务'],
    metrics: () => [
      { label: '本周新增邀请', value: '2,186', note: '示例活动周期' },
      { label: '转化率', value: '18.9%', note: '注册到订阅转化' },
      { label: '单周最高奖励', value: '6,800 USDT', note: '榜单冠军示例' },
      { label: '邀请任务完成率', value: '64%', note: '近四周平均' }
    ],
    sections: [
      {
        type: 'cards',
        title: '奖励机制',
        desc: '将不同转化动作拆分说明，避免与返佣口径混淆。',
        items: [
          { title: '邀请注册', body: '被邀请人完成注册并首次登录后，邀请人即可获得基础积分或现金券奖励。', meta: '拉新奖励' },
          { title: '订阅转化', body: '被邀请人完成订阅后，邀请人可获得更高等级的现金或权益奖励。', meta: '核心奖励' },
          { title: '排行榜加成', body: '按有效邀请人数与转化质量进行周榜排行，榜单前列可获得额外奖金池。', meta: '活动激励' }
        ]
      },
      {
        type: 'list',
        title: '适合谁做邀请',
        desc: '帮助用户快速判断自己是否适合稳定参与邀请机制。',
        items: [
          { title: '活跃老用户', text: '熟悉产品且具备真实使用经验，通常能带来更高质量的转化。' },
          { title: '内容创作者', text: '可将教程、复盘、盯盘视频与注册链接组合传播，形成稳定引流。' },
          { title: '社群主理人', text: '适合结合体验营、训练营与打卡活动开展连续引导。' }
        ]
      }
    ]
  },
  plaza: {
    kicker: 'Plaza',
    title: '广场',
    lead: '围绕市场观点、实盘观察、精选信号与热点标的组织内容流，形成适合高频浏览与即时交流的盘中交流区。',
    highlights: ['热帖榜', '精选观点', '实盘观察', '高频关注'],
    metrics: () => [
      { label: '今日新帖', value: '428', note: '示例社区活跃数据' },
      { label: '热帖互动', value: '9,240', note: '点赞 + 评论 + 转发' },
      { label: '活跃作者', value: '136', note: '24小时' },
      { label: '信号讨论串', value: '57', note: '与 AI 盯盘联动' }
    ],
    sections: [
      {
        type: 'cards',
        title: '热门讨论',
        desc: '以主题卡片承接高热度讨论，突出盘中关注点与互动密度。',
        items: [
          { title: 'BTC 是否进入加速段？', body: '多位交易员围绕盘口主动买量、ETF 资金回流和关键阻力位展开讨论。', meta: '2.1k 浏览' },
          { title: 'SOL 巨鲸回补后还能追吗', body: '围绕鲸鱼进场信号、近三小时成交结构和回撤风险做实盘拆解。', meta: '1.6k 浏览' },
          { title: '异常断层与假突破案例库', body: '社区整理了近两周最典型的盘口断层假突破案例，适合复盘学习。', meta: '980 浏览' }
        ]
      },
      {
        type: 'list',
        title: '广场内容板块',
        desc: '先明确内容结构，后续再接入真实讨论与行情内容流。',
        items: [
          { title: '精选信号', text: '将高质量币种信号自动转化为讨论主题，强化内容与行情联动。' },
          { title: '实盘复盘', text: '鼓励用户沉淀进出场逻辑、失误与修正过程，形成可复用的盘中方法库。' },
          { title: '热点话题', text: '围绕大盘、政策、轮动与链上事件组织主题讨论，提高信息聚合效率。' }
        ]
      }
    ]
  },
  blog: {
    kicker: 'Insights',
    title: '博客',
    lead: '用于沉淀策略方法、终端更新、案例研究与行业观察，承担研究输出、内容检索与专业认知建设。',
    highlights: ['方法论文章', '终端更新', '市场洞察', '案例研究'],
    metrics: () => [
      { label: '月新增文章', value: '18', note: '内容团队 + 嘉宾供稿' },
      { label: '平均阅读完成率', value: '43%', note: '长文内容' },
      { label: '搜索流量占比', value: '31%', note: '博客带来的新访客' },
      { label: '收藏率', value: '12.6%', note: '示例内容运营指标' }
    ],
    sections: [
      {
        type: 'cards',
        title: '推荐文章',
        desc: '用代表性文章结构展示研究方向、方法深度与终端认知。',
        items: [
          { title: '如何用盘口失衡识别假突破', body: '从订单簿倾斜、主动买卖量差和大额挂单撤单节奏拆解常见骗线。', meta: '策略方法论' },
          { title: '交易员版 Dashboard 的设计思路', body: '为什么我们把信号墙、市场列表、下单区和告警区做成一体化屏幕。', meta: '产品设计' },
          { title: '鲸鱼进场信号的 5 个误判场景', body: '并不是所有大单都值得跟，重点在于持续性、位置和成交结构。', meta: '案例复盘' }
        ]
      },
      {
        type: 'faq',
        title: '博客运营说明',
        desc: '明确更新节奏与内容边界，方便后续持续输出。',
        items: [
          { q: '文章多久更新一次？', a: '建议每周至少更新 3 篇，覆盖终端动态、策略方法、行业研究与案例复盘。' },
          { q: '是否支持嘉宾投稿？', a: '支持，优先欢迎真实交易案例、风控经验与具备研究深度的内容。' },
          { q: '是否能跳转到对应功能页？', a: '可以，文章可直接承接到 AI 盯盘、VIP 服务、社区与专题页面。' }
        ]
      }
    ]
  },
  help: {
    kicker: 'Help Center',
    title: '帮助中心',
    lead: '围绕终端接入、功能说明、权限规则与异常排查建立统一知识库，降低学习成本与人工支持压力。',
    highlights: ['快速接入', '账号与订阅', '功能操作', '异常排查'],
    metrics: () => [
      { label: '知识库条目', value: '92', note: '建议后续持续扩充' },
      { label: '自助解决率', value: '61%', note: '无需人工介入' },
      { label: '热门问题', value: '14', note: '过去 7 天高频查询' },
      { label: '工单降幅', value: '23%', note: '帮助中心上线后预估' }
    ],
    sections: [
      {
        type: 'list',
        title: '帮助主题',
        desc: '先建立清晰目录，后续再持续补充终端知识条目。',
        items: [
          { title: '快速开始', text: '说明注册、登录、订阅、页面切换、信号查看与交易面板使用路径。' },
          { title: '权限与套餐', text: '解释公开模式与订阅模式差异、套餐到期、续费与退款相关规则。' },
          { title: '页面问题排查', text: '针对数据缺失、WebSocket 连接异常、按钮无响应与图表未加载提供排查步骤。' }
        ]
      },
      {
        type: 'faq',
        title: '高频问题',
        desc: '优先覆盖最常见的访问、环境与终端加载问题。',
        items: [
          { q: '为什么首页有些币种看不到？', a: '公开模式仅开放部分币种与功能，订阅后解锁完整监控池。' },
          { q: '为什么控制台提示 Origin not allowed？', a: '这类提示通常来自浏览器钱包或第三方扩展注入脚本，并不代表站点主业务请求失败。' },
          { q: '为什么页面样式或脚本 404？', a: '如果遇到 `/static/css/portal.css` 或 `/static/js/app/portal.js` 404，通常说明静态资源未完整发布或缓存仍在读取旧版本。' }
        ]
      }
    ]
  },
  announcements: {
    kicker: 'Announcements',
    title: '公告',
    lead: '作为平台正式信息窗口，用于发布版本更新、维护安排、权限调整与重要风险提示。',
    highlights: ['版本更新', '维护通知', '权限调整', '风险提示'],
    metrics: () => [
      { label: '本月公告', value: '26', note: '运营 + 产品 + 运维' },
      { label: '版本发布', value: '9', note: '本月功能更新' },
      { label: '维护通知', value: '3', note: '计划内维护' },
      { label: '活动预告', value: '7', note: '站内运营节奏' }
    ],
    sections: [
      {
        type: 'timeline',
        title: '最新公告',
        desc: '按时间顺序展示正式公告，便于用户快速确认最近的终端与权限变更。',
        items: [
          { time: '03-24 10:00', title: '站点门户能力上线', text: '新增首页、AI 盯盘、机构服务、内容中心与支持说明等完整导航体系。' },
          { time: '03-23 21:30', title: '订阅权益展示优化', text: '完善套餐矩阵、访问状态提示与订阅后解锁说明，减少权限理解成本。' },
          { time: '03-22 14:00', title: '异常监控能力升级', text: '新增盘口断层、异常刷量与大额撤单识别，提高盘中预警覆盖。' }
        ]
      }
    ]
  },
  news: {
    kicker: 'Newsroom',
    title: '新闻中心',
    lead: '聚合市场焦点、政策动态与专题解读，帮助用户快速理解“发生了什么”以及“对盘中判断意味着什么”。',
    highlights: ['市场快讯', '专题深读', '政策动态', '交易热点'],
    metrics: () => [
      { label: '日均快讯', value: '68', note: '示例内容量' },
      { label: '专题阅读', value: '14.2万', note: '近30天累计' },
      { label: '热点专题', value: '11', note: '本月策划' },
      { label: '新闻到站转化', value: '9.4%', note: '内容带首页流量' }
    ],
    sections: [
      {
        type: 'cards',
        title: '今日焦点',
        desc: '以焦点卡片形式承接高关注市场事件与专题解读，服务交易与研究跟踪。',
        items: [
          { title: 'BTC 再次测试关键压力位，AI 盯盘信号同步升温', body: '结合盘口主动买量与鲸鱼行为，平台将其列入首页重点关注列表。', meta: '市场焦点' },
          { title: '山寨轮动加剧，如何从信号墙筛选高质量标的', body: '从单纯追涨切换到观察成交结构、异常信号与鲸鱼行为的组合筛选方式。', meta: '专题解析' },
          { title: '交易员为什么需要一体化盯盘界面', body: '当信号、盘口、交易与告警分散在多个页面时，执行效率和判断连续性都会明显下降。', meta: '深度观察' }
        ]
      }
    ]
  },
  community: {
    kicker: 'Community',
    title: '社区',
    lead: '用于承接交易讨论群、主题群与合作伙伴网络，是盘中交流、内容扩散与长期运营的重要入口。',
    highlights: ['官方群矩阵', '主题社群', '盘中交流', '合作伙伴网络'],
    metrics: () => [
      { label: '社群总人数', value: '48,600+', note: '示例累计数据' },
      { label: '日活跃发言', value: '8,300+', note: '多群合计' },
      { label: 'AMA 场次', value: '22', note: '近 60 天' },
      { label: '社群转订阅', value: '14.7%', note: '内容导向转化' }
    ],
    sections: [
      {
        type: 'cards',
        title: '社区组成',
        desc: '先定义不同社群的角色定位，便于后续持续维护。',
        items: [
          { title: '官方公告群', body: '同步版本更新、重要活动、系统维护与关键通知，保证信息触达一致。', meta: '信息同步' },
          { title: '交易讨论群', body: '围绕热点币种、AI 信号、实盘复盘与策略判断展开高频交流。', meta: '核心用户' },
          { title: '合作伙伴群', body: '服务渠道、研究伙伴、项目方、代理与机构客户的商务对接与联合运营。', meta: '商务拓展' }
        ]
      },
      {
        type: 'faq',
        title: '社区运营说明',
        desc: '提前说明规则与边界，降低后续治理成本。',
        items: [
          { q: '社区是否允许发广告？', a: '普通讨论群不开放自由广告发布，合作需求请通过广告页或商务渠道申请。' },
          { q: '是否有地区或语言分群？', a: '可以逐步扩展为中文主群、英文群、区域群与主题群的分层结构。' },
          { q: '是否有官方直播或活动？', a: '建议与公告、广场和博客联动，形成固定频次的内容与活动节奏。' }
        ]
      }
    ]
  },
  agreement: {
    kicker: 'Legal',
    title: '服务协议',
    lead: '以可读化方式说明平台提供的终端服务边界、用户义务、风险揭示与责任限制，帮助用户快速理解关键条款。',
    highlights: ['服务边界', '用户义务', '风险揭示', '责任限制'],
    metrics: () => [
      { label: '协议版本', value: 'v1.2.0', note: '示例法务版本号' },
      { label: '最近更新', value: '2026-03-24', note: '本次站点页同步' },
      { label: '核心条款', value: '8', note: '可读化分节展示' },
      { label: '适用范围', value: '全站用户', note: '注册、登录、访问、订阅' }
    ],
    sections: [
      {
        type: 'list',
        title: '核心条款摘要',
        desc: '先提供用户可快速理解的摘要版本，后续可补充完整法务文本。',
        items: [
          { title: '信息服务属性', text: '平台提供数据展示、分析信号、内容服务与相关工具，不构成收益承诺或投资建议。' },
          { title: '账户安全责任', text: '用户需妥善保管账户凭据，不得共享、转售、盗用或实施破坏性访问行为。' },
          { title: '风险自担', text: '所有交易行为均由用户独立决策并自行承担风险，平台不对市场波动或第三方平台风险负责。' },
          { title: '违规处理', text: '对刷号、滥用、违法内容、恶意攻击等行为，平台保留限制、终止服务与追责权利。' }
        ]
      }
    ]
  },
  privacy: {
    kicker: 'Privacy',
    title: '隐私说明',
    lead: '围绕收集范围、处理目的、安全措施与用户权利四个方面，说明平台如何处理和保护终端相关数据。',
    highlights: ['收集范围', '处理目的', '安全措施', '用户权利'],
    metrics: () => [
      { label: '数据分类', value: '4 类', note: '账号、设备、行为、订阅' },
      { label: '安全策略', value: '最小权限', note: '示例治理原则' },
      { label: '保留策略', value: '按业务分级', note: '示例说明' },
      { label: '用户权利', value: '查询 / 更正 / 删除', note: '可申请处理' }
    ],
    sections: [
      {
        type: 'list',
        title: '隐私要点',
        desc: '用清晰直白的方式说明用户最关心的终端数据问题。',
        items: [
          { title: '我们收集什么', text: '包括账户信息、订阅记录、页面使用日志和用于保障稳定性的必要技术信息。' },
          { title: '为什么收集', text: '用于提供功能、保障安全、改进终端、处理工单，并完成订阅与相关服务流程。' },
          { title: '如何保护', text: '通过权限控制、日志审计、分级存储与必要的传输保护措施降低数据风险。' },
          { title: '你能做什么', text: '可申请查询、更正、注销或删除部分个人信息，法律法规另有规定的除外。' }
        ]
      }
    ]
  },
  about: {
    kicker: 'About BB-Market',
    title: '关于我们',
    lead: '围绕产品定位、服务对象与长期交付方向，说明团队为什么做这套终端，以及它希望解决什么问题。',
    highlights: ['交易员视角', '数据驱动', '终端化交付', '长期产品化'],
    metrics: () => [
      { label: '产品方向', value: '专业终端 + 内容能力', note: '双线协同' },
      { label: '覆盖场景', value: '盯盘 / 交易 / 内容 / 商务', note: '统一体系' },
      { label: '迭代节奏', value: '周更', note: '持续演进' },
      { label: '当前版本', value: 'Portal + Dashboard', note: '前台与终端一体化' }
    ],
    sections: [
      {
        type: 'cards',
        title: '我们在做什么',
        desc: '从产品价值、交付方式与终端结构三个角度定义 BB-Market。',
        items: [
          { title: '构建交易员愿意长期打开的终端', body: '将关键市场判断、执行入口与复盘能力收敛到同一界面，而不是堆砌孤立指标。', meta: '核心产品观' },
          { title: '构建研究与工具协同的体系', body: '让用户在查看信号之外，还能获取资讯、案例、专题与合作入口，形成完整使用闭环。', meta: '终端结构' },
          { title: '构建可持续交付的业务体系', body: '让每个页面都承担明确角色，既服务前台表达，也支持销售转化与长期运营。', meta: '交付方向' }
        ]
      },
      {
        type: 'list',
        title: '下一阶段重点',
        desc: '说明下一阶段的终端建设重点与落地顺序。',
        items: [
          { title: '接入真实内容与数据源', text: '逐步把博客、新闻、公告、广场与社区接入正式后台与审核流程。' },
          { title: '补充正式提交与线索承接能力', text: '为反馈、广告、VIP 与合作页面补充表单、工单与 CRM 承接链路。' },
          { title: '继续完善终端与首页表达', text: '优化移动端、响应式与不同权限状态下的内容与功能展示。' }
        ]
      }
    ]
  }
};

const HOME_PARTNERS = ['Binance', 'OKX', 'Bybit', 'TradingView', 'Telegram', 'Notion', 'Webhook', 'Desk API'];
const HOME_HERO_ROTATE_MS = 3800;
const HOME_BANNER_ROTATE_MS = 5200;

function escapePortalHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function portalRuntimeMetrics() {
  const syms = Array.isArray(S.syms) ? S.syms : [];
  const feed = Array.isArray(S.feed) ? S.feed : [];
  const plans = Array.isArray(S.auth?.plans) ? S.auth.plans : [];
  const strongSignals = syms.filter(s => Math.max(sv(s.symbol, 'ps'), sv(s.symbol, 'ds')) >= 70).length;
  const whales = syms.filter(s => s.whale_entry || s.whale_exit).length;
  return {
    totalSymbols: syms.length || S.access?.total_symbols || 128,
    visibleSymbols: S.access?.visible_symbols || Math.min(syms.length || 24, 10),
    strongSignals: strongSignals || 19,
    whales: whales || 12,
    feedCount: feed.length || 186,
    planCount: plans.length || 3,
    userLabel: S.auth?.user ? (S.auth.user.display_name || S.auth.user.username) : '访客',
    accessLabel: S.access?.full_access ? '已订阅' : (S.auth?.user ? '已登录未订阅' : '公开预览'),
    subscriptionPlan: S.access?.subscription_plan || (S.access?.full_access ? 'pro_month' : 'public'),
    expiresAt: S.access?.subscription_expires_at || '未订阅'
  };
}

function portalPulseItems() {
  const syms = [...(S.syms || [])];
  const scored = syms
    .map(s => {
      const pump = sv(s.symbol, 'ps');
      const dump = sv(s.symbol, 'ds');
      const score = Math.max(pump, dump);
      const rising = pump >= dump;
      return {
        symbol: s.symbol,
        score: Math.round(score),
        tag: s.watch_level || (rising ? '上涨监控' : '下跌监控'),
        change: `${(s.change_24h_pct || 0) >= 0 ? '+' : ''}${(s.change_24h_pct || 0).toFixed(2)}%`
      };
    })
    .sort((a, b) => b.score - a.score)
    .slice(0, 5);
  return scored.length ? scored : PORTAL_FALLBACK_PULSE;
}

function renderPortalActions(page) {
  const actions = [];
  if (page !== 'home') {
    actions.push({ label: '返回首页', action: "switchSitePage('home')", kind: 'primary' });
  }
  if (!S.auth?.user) {
    actions.push({ label: '注册体验', action: "openAuthModal('register')", kind: 'secondary' });
  } else if (!S.access?.full_access) {
    actions.push({ label: '立即订阅解锁', action: 'subscribeNow()', kind: 'secondary' });
  } else {
    actions.push({ label: '查看 VIP 权益', action: "switchSitePage('vip')", kind: 'secondary' });
  }
  return `<div class="portal-actions">${actions.map(item=>`<button class="portal-btn ${item.kind}" type="button" onclick="${item.action}">${escapePortalHtml(item.label)}</button>`).join('')}</div>`;
}

function renderPortalMetrics(items) {
  return `<div class="portal-metrics">${items.map(item=>`
    <div class="portal-metric-card">
      <div class="portal-metric-label">${escapePortalHtml(item.label)}</div>
      <div class="portal-metric-value">${escapePortalHtml(item.value)}</div>
      <div class="portal-metric-note">${escapePortalHtml(item.note)}</div>
    </div>
  `).join('')}</div>`;
}

function renderPortalCardsSection(section) {
  return `
    <section class="portal-section">
      <div class="portal-section-head">
        <div class="portal-section-title">${escapePortalHtml(section.title)}</div>
        <div class="portal-section-desc">${escapePortalHtml(section.desc || '')}</div>
      </div>
      <div class="portal-card-grid">
        ${section.items.map(item=>`
          <article class="portal-card">
            <div class="portal-card-title">${escapePortalHtml(item.title)}</div>
            <div class="portal-card-body">${escapePortalHtml(item.body)}</div>
            <div class="portal-card-meta">${escapePortalHtml(item.meta || '')}</div>
          </article>
        `).join('')}
      </div>
    </section>
  `;
}

function renderPortalListSection(section) {
  return `
    <section class="portal-section">
      <div class="portal-section-head">
        <div class="portal-section-title">${escapePortalHtml(section.title)}</div>
        <div class="portal-section-desc">${escapePortalHtml(section.desc || '')}</div>
      </div>
      <div class="portal-list">
        ${section.items.map(item=>`
          <article class="portal-list-item">
            <div class="portal-list-title">${escapePortalHtml(item.title)}</div>
            <div class="portal-list-text">${escapePortalHtml(item.text)}</div>
          </article>
        `).join('')}
      </div>
    </section>
  `;
}

function renderPortalFaqSection(section) {
  return `
    <section class="portal-section">
      <div class="portal-section-head">
        <div class="portal-section-title">${escapePortalHtml(section.title)}</div>
        <div class="portal-section-desc">${escapePortalHtml(section.desc || '')}</div>
      </div>
      <div class="portal-faq">
        ${section.items.map(item=>`
          <article class="portal-faq-item">
            <div class="portal-faq-q">${escapePortalHtml(item.q)}</div>
            <div class="portal-faq-a">${escapePortalHtml(item.a)}</div>
          </article>
        `).join('')}
      </div>
    </section>
  `;
}

function renderPortalTableSection(section) {
  return `
    <section class="portal-section">
      <div class="portal-section-head">
        <div class="portal-section-title">${escapePortalHtml(section.title)}</div>
        <div class="portal-section-desc">${escapePortalHtml(section.desc || '')}</div>
      </div>
      <div class="portal-table-wrap">
        <table class="portal-table">
          <thead>
            <tr>${section.columns.map(col=>`<th>${escapePortalHtml(col)}</th>`).join('')}</tr>
          </thead>
          <tbody>
            ${section.rows.map(row=>`<tr>${row.map(cell=>`<td>${escapePortalHtml(cell)}</td>`).join('')}</tr>`).join('')}
          </tbody>
        </table>
      </div>
    </section>
  `;
}

function renderPortalTimelineSection(section) {
  return `
    <section class="portal-section">
      <div class="portal-section-head">
        <div class="portal-section-title">${escapePortalHtml(section.title)}</div>
        <div class="portal-section-desc">${escapePortalHtml(section.desc || '')}</div>
      </div>
      <div class="portal-timeline">
        ${section.items.map(item=>`
          <article class="portal-timeline-item">
            <div class="portal-timeline-time">${escapePortalHtml(item.time)}</div>
            <div class="portal-timeline-title">${escapePortalHtml(item.title)}</div>
            <div class="portal-timeline-text">${escapePortalHtml(item.text)}</div>
          </article>
        `).join('')}
      </div>
    </section>
  `;
}

function renderPortalSection(section) {
  if (section.type === 'cards') return renderPortalCardsSection(section);
  if (section.type === 'list') return renderPortalListSection(section);
  if (section.type === 'faq') return renderPortalFaqSection(section);
  if (section.type === 'table') return renderPortalTableSection(section);
  if (section.type === 'timeline') return renderPortalTimelineSection(section);
  return '';
}

function renderPortalSidebar(runtime) {
  const pulse = portalPulseItems();
  return `
    <aside class="portal-sidebar">
      <section class="portal-sidecard">
        <div class="portal-sidecard-title">访问状态</div>
        <div class="portal-sidecard-kv"><span>当前用户</span><b>${escapePortalHtml(runtime.userLabel)}</b></div>
        <div class="portal-sidecard-kv"><span>访问等级</span><b>${escapePortalHtml(runtime.accessLabel)}</b></div>
        <div class="portal-sidecard-kv"><span>可见币种</span><b>${escapePortalHtml(runtime.visibleSymbols)} / ${escapePortalHtml(runtime.totalSymbols)}</b></div>
        <div class="portal-sidecard-kv"><span>当前套餐</span><b>${escapePortalHtml(runtime.subscriptionPlan)}</b></div>
        <div class="portal-sidecard-kv"><span>到期时间</span><b>${escapePortalHtml(runtime.expiresAt)}</b></div>
      </section>
      <section class="portal-sidecard">
        <div class="portal-sidecard-title">实时热度</div>
        <div class="portal-pulse">
          ${pulse.map(item=>`
            <div class="portal-pulse-item">
              <div>
                <div class="portal-pulse-symbol">${escapePortalHtml(item.symbol)}</div>
                <div class="portal-pulse-tag">${escapePortalHtml(item.tag)}</div>
              </div>
              <div class="portal-pulse-right">
                <div class="portal-pulse-score">${escapePortalHtml(item.score)}</div>
                <div class="portal-pulse-change">${escapePortalHtml(item.change)}</div>
              </div>
            </div>
          `).join('')}
        </div>
      </section>
      <section class="portal-sidecard">
        <div class="portal-sidecard-title">快速导航</div>
        <div class="portal-quicklinks">
          <button type="button" onclick="switchSitePage('ai')">AI盯盘</button>
          <button type="button" onclick="switchSitePage('vip')">VIP服务</button>
          <button type="button" onclick="switchSitePage('ads')">广告</button>
          <button type="button" onclick="switchSitePage('feedback')">反馈</button>
          <button type="button" onclick="switchSitePage('community')">社区</button>
          <button type="button" onclick="switchSitePage('about')">关于我们</button>
        </div>
      </section>
    </aside>
  `;
}

function renderPortalFooter(page) {
  return `
    <footer class="portal-footer">
      <div class="portal-footer-main">
        ${PORTAL_FOOTER_GROUPS.map(group=>`
          <div class="portal-footer-group">
            <div class="portal-footer-group-title">${escapePortalHtml(group.title)}</div>
            <div class="portal-footer-links">
              ${group.links.map(link=>`
                <button
                  type="button"
                  class="portal-footer-link ${page===link.page?'act':''}"
                  data-page="${escapePortalHtml(link.page)}"
                  onclick="switchSitePage('${escapePortalHtml(link.page)}',this)"
                >${escapePortalHtml(link.label)}</button>
              `).join('')}
            </div>
          </div>
        `).join('')}
      </div>
    </footer>
  `;
}

function setPortalText(id, text) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = String(text ?? '');
}

function setPortalHtml(id, html) {
  const el = document.getElementById(id);
  if (!el) return;
  el.innerHTML = String(html ?? '');
}

function pulsePortalNode(id, text) {
  const el = document.getElementById(id);
  if (!el) return;
  const next = String(text ?? '');
  if (el.textContent === next) return;
  el.textContent = next;
  el.classList.remove('is-live-bump');
  void el.offsetWidth;
  el.classList.add('is-live-bump');
}

function homeSpotlightItems() {
  const base = portalPulseItems();
  return base.length ? base : PORTAL_FALLBACK_PULSE;
}

function renderHomeSpotlightNav(items, activeIndex = 0) {
  return items.slice(0, 5).map((item, index) => `
    <button
      type="button"
      class="home-spot-chip ${index===activeIndex?'act':''}"
      onclick="selectHomeSpotlight(${index})"
    >
      <span>${escapePortalHtml(item.symbol)}</span>
      <b>${escapePortalHtml(String(item.score))}</b>
    </button>
  `).join('');
}

function renderHomeLiveTape(items) {
  const list = (items && items.length ? items : [
    { symbol: 'BTCUSDT', type: '盘口异动', desc: '主动买盘持续抬升，关注突破确认。' },
    { symbol: 'ETHUSDT', type: '预警通知', desc: '短时成交密度提升，波动加快。' },
    { symbol: 'SOLUSDT', type: '鲸鱼信号', desc: '连续大额扫单，注意后续承接。' }
  ]);
  const cards = list.map(item=>`
    <div class="home-live-card">
      <span class="home-live-symbol">${escapePortalHtml(item.symbol)}</span>
      <span class="home-live-type">${escapePortalHtml(item.type || '实时信号')}</span>
      <span class="home-live-desc">${escapePortalHtml(item.desc || '市场信号持续刷新中')}</span>
    </div>
  `).join('');
  return `
    <div class="home-live-track">
      ${cards}
      ${cards}
    </div>
  `;
}

function renderHomePartnerRail() {
  const cells = HOME_PARTNERS.map(name=>`
    <span class="home-partner-pill">${escapePortalHtml(name)}</span>
  `).join('');
  return `
    <div class="home-partner-rail">
      <div class="home-partner-track">
        ${cells}
        ${cells}
      </div>
    </div>
  `;
}

function renderHomeHeroBackdrop() {
  const particles = Array.from({ length: 14 }, (_, index) => {
    const left = 6 + (index * 7) % 92;
    const top = 8 + (index * 11) % 76;
    const delay = (index * 0.45).toFixed(2);
    const size = 2 + (index % 4);
    return `<span class="home-hero-particle" style="--x:${left}%;--y:${top}%;--d:${delay}s;--s:${size}px"></span>`;
  }).join('');
  return `
    <div class="home-hero-backdrop" aria-hidden="true">
      <div class="home-hero-grid"></div>
      <div class="home-hero-scan"></div>
      <div class="home-hero-particles">${particles}</div>
    </div>
  `;
}

function renderHomeArchitecture(runtime, pulse) {
  const primary = pulse[0] || PORTAL_FALLBACK_PULSE[0];
  return `
    <section class="home-architecture home-reveal" data-reveal-delay="0.08">
      <div class="home-section-head">
        <div class="home-section-kicker">系统架构</div>
        <div class="home-section-title">把行情接入、因子计算、预警分发与交易执行，组织成适用于交易席位与量化团队的数据链路</div>
      </div>
      <div class="home-arch-layout">
        <div class="home-arch-board">
          <div class="home-arch-line" aria-hidden="true"></div>
          <article class="home-arch-node home-reveal is-node-accent" data-reveal-delay="0.14">
            <div class="home-arch-node-kicker">输入层</div>
            <div class="home-arch-node-title">行情接入层</div>
            <div class="home-arch-node-text">盘口、成交、K线与异动事件统一接入，形成连续、可追踪的实时数据流。</div>
          </article>
          <article class="home-arch-node home-reveal" data-reveal-delay="0.22">
            <div class="home-arch-node-kicker">计算层</div>
            <div class="home-arch-node-title">因子计算层</div>
            <div class="home-arch-node-text">OBI、OFI、主动买卖量差、鲸鱼轨迹与异常结构在这里收敛为统一评分。</div>
          </article>
          <article class="home-arch-node home-reveal is-node-primary" data-reveal-delay="0.30">
            <div class="home-arch-node-kicker">决策层</div>
            <div class="home-arch-node-title">决策引擎</div>
            <div class="home-arch-node-text">将复杂市场结构转译为可执行的强提醒、关注理由与优先级排序。</div>
          </article>
          <article class="home-arch-node home-reveal" data-reveal-delay="0.38">
            <div class="home-arch-node-kicker">分发层</div>
            <div class="home-arch-node-title">预警分发层</div>
            <div class="home-arch-node-text">实时信号、预警通知、回放链路与订阅权限在这里完成统一路由。</div>
          </article>
          <article class="home-arch-node home-reveal is-node-accent-alt" data-reveal-delay="0.46">
            <div class="home-arch-node-kicker">执行层</div>
            <div class="home-arch-node-title">交易工作台</div>
            <div class="home-arch-node-text">最终落到 AI 盯盘终端、交易面板与复盘流程，形成面向席位执行的完整闭环。</div>
          </article>
        </div>
        <div class="home-arch-side home-reveal" data-reveal-delay="0.18">
          <div class="home-arch-side-card">
            <div class="home-side-kicker">当前优先监控标的</div>
            <div class="home-side-title">${escapePortalHtml(primary.symbol)}</div>
            <div class="home-side-text">${escapePortalHtml(primary.tag)}，系统会将该标的优先推进到席位终端与预警事件流中。</div>
          </div>
          <div class="home-arch-side-grid">
            <div class="home-arch-mini">
              <b>${escapePortalHtml(String(runtime.totalSymbols))}</b>
              <span>统一监控币种</span>
            </div>
            <div class="home-arch-mini">
              <b>${escapePortalHtml(String(runtime.strongSignals))}</b>
              <span>高优先级信号</span>
            </div>
            <div class="home-arch-mini">
              <b>${escapePortalHtml(String(runtime.whales))}</b>
              <span>鲸鱼异动捕捉</span>
            </div>
            <div class="home-arch-mini">
              <b>${escapePortalHtml(String(runtime.feedCount))}</b>
              <span>实时事件沉淀</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  `;
}

function observeHomeReveal() {
  const portal = document.getElementById('portal-shell');
  if (!portal || S.site?.page !== 'home') return;
  const items = portal.querySelectorAll('.home-reveal');
  if (!items.length) return;
  const reduceMotion = typeof window.matchMedia === 'function'
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reduceMotion || typeof IntersectionObserver === 'undefined') {
    items.forEach(node => node.classList.add('is-visible'));
    return;
  }
  if (window.__bbHomeRevealObserver) {
    try { window.__bbHomeRevealObserver.disconnect(); } catch (_) {}
  }
  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (!entry.isIntersecting) return;
      const node = entry.target;
      const delay = Number(node.dataset.revealDelay || 0);
      node.style.transitionDelay = `${delay}s`;
      node.classList.add('is-visible');
      observer.unobserve(node);
    });
  }, { threshold: 0.16, rootMargin: '0px 0px -8% 0px' });
  items.forEach(node => observer.observe(node));
  window.__bbHomeRevealObserver = observer;
}

function renderHomeMetricCards(items, valueClass = 'home-stat-value') {
  return items.map((item, index) => `
    <article class="home-stat-card">
      <div class="${valueClass}" id="${valueClass}-${index}">${escapePortalHtml(item.value)}</div>
      <div class="home-stat-label">${escapePortalHtml(item.label)}</div>
      <div class="home-stat-note">${escapePortalHtml(item.note)}</div>
      <div class="home-stat-bar"><span style="width:${Math.max(24,Math.min(100,32 + index*16))}%"></span></div>
    </article>
  `).join('');
}

function svgToDataUri(svg) {
  return `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(svg)
    .replace(/%0A/g, '')
    .replace(/%20/g, ' ')}`;
}

function bannerTextUnits(text = '') {
  return [...String(text || '')].reduce((sum, ch) => {
    if (/\s/.test(ch)) return sum + 0.28;
    return sum + (/[\u0000-\u00ff]/.test(ch) ? 0.56 : 1);
  }, 0);
}

function wrapBannerSvgText(text, maxUnitsPerLine, maxLines = 2) {
  const source = String(text || '').trim();
  if (!source) return [''];
  const lines = [];
  let line = '';
  let units = 0;
  [...source].forEach(ch => {
    const nextUnits = bannerTextUnits(ch);
    if (line && units + nextUnits > maxUnitsPerLine && lines.length < maxLines - 1) {
      lines.push(line.trim());
      line = ch;
      units = nextUnits;
      return;
    }
    line += ch;
    units += nextUnits;
  });
  if (line) lines.push(line.trim());
  if (lines.length > maxLines) {
    const head = lines.slice(0, maxLines - 1);
    const tail = lines.slice(maxLines - 1).join('');
    return [...head, tail];
  }
  return lines;
}

function renderBannerSvgText(x, y, lines, options = {}) {
  const {
    fill = '#FFFFFF',
    fontSize = 28,
    fontWeight = 800,
    lineHeight = fontSize * 1.1,
    letterSpacing = 0
  } = options;
  return `
    <text x="${x}" y="${y}" fill="${fill}" font-size="${fontSize}" font-weight="${fontWeight}"${letterSpacing ? ` letter-spacing="${letterSpacing}"` : ''}>
      ${lines.map((line, index) => `<tspan x="${x}" dy="${index === 0 ? 0 : lineHeight}">${escapePortalHtml(line)}</tspan>`).join('')}
    </text>
  `;
}

function buildHomeBannerSvg(spec) {
  const titleLines = wrapBannerSvgText(spec.title, 10.2, 2);
  const titleFont = titleLines.length > 1 ? 50 : 66;
  const titleLineHeight = titleLines.length > 1 ? 52 : 68;
  const titleY = titleLines.length > 1 ? 186 : 212;
  const subtitleLines = wrapBannerSvgText(spec.subtitle, 24, 2);
  const subtitleFont = subtitleLines.length > 1 ? 18 : 24;
  const subtitleLineHeight = subtitleLines.length > 1 ? 24 : 28;
  const subtitleY = titleY + (titleLines.length - 1) * titleLineHeight + (titleLines.length > 1 ? 56 : 46);
  const contentBottom = subtitleY + (subtitleLines.length - 1) * subtitleLineHeight;
  const panelTop = Math.min(368, contentBottom + 40);
  const panelShift = panelTop - 330;
  const bars = (spec.bars || []).map((value, index) => {
    const height = Math.max(30, Math.min(124, Number(value) || 48));
    const x = 86 + index * 54;
    const y = 192 - height;
    const pulse = 8 + (index % 3) * 6;
    return `
      <rect x="${x}" y="${y}" width="28" height="${height}" rx="10" fill="${index === 0 ? spec.accent : 'rgba(255,255,255,0.18)'}">
        <animate attributeName="y" values="${y};${Math.max(54, y - pulse)};${y}" dur="${2.6 + index * 0.18}s" repeatCount="indefinite"/>
        <animate attributeName="height" values="${height};${Math.min(148, height + pulse)};${height}" dur="${2.6 + index * 0.18}s" repeatCount="indefinite"/>
      </rect>
    `;
  }).join('');
  const particles = (spec.particles || [
    { cx: 984, cy: 208, r: 10, dx: 14, dy: -10, delay: '0s' },
    { cx: 1096, cy: 266, r: 8, dx: -12, dy: 14, delay: '0.4s' },
    { cx: 1210, cy: 194, r: 12, dx: 16, dy: 12, delay: '0.8s' },
    { cx: 1316, cy: 286, r: 9, dx: -10, dy: -14, delay: '1.2s' },
    { cx: 1434, cy: 226, r: 7, dx: 12, dy: 10, delay: '1.6s' }
  ]).map(item => `
    <circle cx="${item.cx}" cy="${item.cy}" r="${item.r}" fill="${spec.accent}" fill-opacity="0.26">
      <animate attributeName="cx" values="${item.cx};${item.cx + item.dx};${item.cx}" dur="4.8s" begin="${item.delay}" repeatCount="indefinite"/>
      <animate attributeName="cy" values="${item.cy};${item.cy + item.dy};${item.cy}" dur="4.8s" begin="${item.delay}" repeatCount="indefinite"/>
      <animate attributeName="r" values="${item.r};${item.r + 3};${item.r}" dur="3.4s" begin="${item.delay}" repeatCount="indefinite"/>
      <animate attributeName="fill-opacity" values="0.12;0.34;0.12" dur="3.4s" begin="${item.delay}" repeatCount="indefinite"/>
    </circle>
  `).join('');
  const chips = (spec.chips || []).slice(0, 3).map((item, index) => `
    <g transform="translate(${700 + index * 176},356)">
      <rect width="164" height="36" rx="18" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.12)"/>
      <text x="82" y="24" text-anchor="middle" fill="#F3F6FB" font-size="13" font-weight="700">${item}</text>
    </g>
  `).join('');
  const cards = (spec.cards || []).slice(0, 3).map((item, index) => `
    <g transform="translate(${678 + index * 244},406)">
      <rect width="228" height="118" rx="22" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.10)"/>
      <text x="20" y="32" fill="#9FB0C7" font-size="14" font-weight="700">${item.label}</text>
      <text x="20" y="74" fill="#FFFFFF" font-size="36" font-weight="800">${item.value}
        <animateTransform attributeName="transform" type="translate" values="0 0;0 -4;0 0" dur="${1.7 + index * 0.2}s" repeatCount="indefinite"/>
        <animate attributeName="opacity" values="0.82;1;0.82" dur="${1.7 + index * 0.2}s" repeatCount="indefinite"/>
      </text>
      <text x="20" y="98" fill="${spec.accent}" font-size="13" font-weight="700">${item.note}</text>
    </g>
  `).join('');
  const svg = `
    <svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900" viewBox="0 0 1600 900" fill="none">
      <defs>
        <linearGradient id="bg-${spec.key}" x1="140" y1="100" x2="1380" y2="820" gradientUnits="userSpaceOnUse">
          <stop stop-color="${spec.bgStart}"/>
          <stop offset="1" stop-color="${spec.bgEnd}"/>
        </linearGradient>
        <linearGradient id="glow-${spec.key}" x1="0" y1="0" x2="1" y2="1">
          <stop stop-color="${spec.accent}"/>
          <stop offset="1" stop-color="rgba(255,255,255,0.08)"/>
        </linearGradient>
      </defs>
      <rect width="1600" height="900" rx="42" fill="url(#bg-${spec.key})"/>
      <circle cx="1280" cy="170" r="210" fill="${spec.glow}" fill-opacity="0.20"/>
      <circle cx="236" cy="726" r="244" fill="${spec.glow}" fill-opacity="0.12"/>
      <rect x="-220" y="82" width="240" height="736" fill="rgba(255,255,255,0.08)" opacity="0.22" transform="rotate(10 0 0)">
        <animate attributeName="x" values="-260;1600" dur="5.2s" repeatCount="indefinite"/>
      </rect>
      <g opacity="0.16" stroke="rgba(255,255,255,0.14)">
        <path d="M0 140H1600"/>
        <path d="M0 300H1600"/>
        <path d="M0 460H1600"/>
        <path d="M0 620H1600"/>
        <path d="M220 0V900"/>
        <path d="M420 0V900"/>
        <path d="M620 0V900"/>
        <path d="M820 0V900"/>
        <path d="M1020 0V900"/>
        <path d="M1220 0V900"/>
        <path d="M1420 0V900"/>
      </g>
      <rect x="72" y="72" width="1456" height="756" rx="34" fill="rgba(7,10,15,0.26)" stroke="rgba(255,255,255,0.10)"/>
      ${particles}
      <text x="116" y="144" fill="#F2C760" font-size="28" font-weight="800" letter-spacing="4">${spec.kicker}</text>
      ${renderBannerSvgText(116, titleY, titleLines, { fill: '#F5F7FB', fontSize: titleFont, fontWeight: 900, lineHeight: titleLineHeight })}
      ${renderBannerSvgText(116, subtitleY, subtitleLines, { fill: '#C8D1DC', fontSize: subtitleFont, fontWeight: 500, lineHeight: subtitleLineHeight })}
      <g transform="translate(0, ${panelShift})">
        <rect x="104" y="330" width="474" height="264" rx="28" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.12)"/>
        <text x="138" y="382" fill="#9FB0C7" font-size="20" font-weight="700">${spec.panelTitle}</text>
        ${bars}
        <path d="${spec.linePath}" stroke="${spec.accent}" stroke-width="8" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="460" stroke-dashoffset="460">
          <animate attributeName="stroke-dashoffset" values="460;0;0" dur="2.8s" repeatCount="indefinite"/>
        </path>
        <circle cx="302" cy="436" r="10" fill="${spec.accent}"/>
        <circle cx="408" cy="384" r="10" fill="${spec.accent}"/>
        <circle cx="516" cy="344" r="10" fill="${spec.accent}"/>
        <rect x="650" y="330" width="774" height="264" rx="28" fill="rgba(255,255,255,0.04)" stroke="rgba(255,255,255,0.10)"/>
        ${chips}
        ${cards}
        <rect x="104" y="646" width="1316" height="128" rx="26" fill="rgba(255,255,255,0.04)" stroke="rgba(255,255,255,0.08)"/>
        <text x="138" y="700" fill="#FFFFFF" font-size="34" font-weight="800">${spec.footerTitle}</text>
        <text x="138" y="740" fill="#C8D1DC" font-size="22">${spec.footerText}</text>
        <rect x="1122" y="676" width="262" height="56" rx="28" fill="url(#glow-${spec.key})"/>
        <text x="1253" y="712" text-anchor="middle" fill="#101318" font-size="20" font-weight="900">${spec.bannerCta || spec.ctaLabel || 'BB-Market'}</text>
      </g>
    </svg>`;
  return svgToDataUri(svg);
}

function homeBannerSlides(runtime) {
  const isAuthenticated = !!S.auth?.user;
  const hasFullAccess = !!S.access?.full_access;
  const accessLabel = hasFullAccess ? '全量访问已开通' : (isAuthenticated ? '已登录，待升级权限' : '公开预览');
  const primaryBannerCta = hasFullAccess
    ? { label: '进入专业终端', action: "switchSitePage('ai')" }
    : (isAuthenticated
      ? { label: '升级机构权限', action: "switchSitePage('vip')" }
      : { label: '申请试用席位', action: "openAuthModal('register')" });
  const secondaryBannerCta = hasFullAccess
    ? { label: '查看机构方案', action: "switchSitePage('vip')" }
    : (isAuthenticated
      ? { label: '进入 AI 盯盘', action: "switchSitePage('ai')" }
      : { label: '登录已有席位', action: "openAuthModal('login')" });
  return [
    {
      key: 'signals',
      kicker: '实时信号体系',
      eyebrow: '实时信号中枢',
      title: hasFullAccess ? '完整实时信号体系已就绪' : (isAuthenticated ? '已登录，下一步解锁席位级信号覆盖' : '让强提醒先于市场波动出现'),
      lead: hasFullAccess
        ? '当前账户已具备完整访问权限，可直接进入 AI 盯盘终端执行盘中决策。'
        : (isAuthenticated
          ? '当前账户已进入系统访问态，继续升级即可解锁更多标的、完整推送与席位级信号能力。'
          : '将拉升、回落、成交节奏与盘口异动压缩为交易席位可直接采用的首屏判断。'),
      stateLabel: accessLabel,
      bgStart: '#0A1016',
      bgEnd: '#132231',
      accent: '#F2C760',
      glow: '#1890FF',
      panelTitle: '实时信号看板',
      subtitle: `覆盖 ${runtime.totalSymbols}+ 个交易标的，持续刷新 ${runtime.feedCount}+ 条盘中事件`,
      linePath: 'M194 486 C246 454 302 438 356 418 C394 404 438 392 470 374 C500 358 530 340 546 320',
      footerTitle: `信号识别、结构确认与执行入口在同一专业终端协同`,
      footerText: '减少在行情页、预警页与成交页之间来回切换，首屏直接呈现交易席位视角。',
      bannerCta: '实时信号',
      bars: [58, 82, 96, 72, 114, 94, 126],
      chips: ['信号看板', '深度结构', '预警引擎'],
      cards: [
        { label: '强提醒候选', value: `${runtime.strongSignals}`, note: '盘中优先级' },
        { label: '事件流', value: `${runtime.feedCount}+`, note: '实时刷新' },
        { label: '响应', value: '秒级', note: '不等全页' }
      ]
    },
    {
      key: 'whales',
      kicker: '大额行为与盘口',
      eyebrow: '鲸鱼与盘口联动',
      title: isAuthenticated ? '把大额行为与盘口确认合并到同一层' : '把大额资金轨迹放回盘口上下文',
      lead: isAuthenticated
        ? '登录后可继续提升订单簿、最新成交与大额行为联动的判断深度，降低只看单笔大单的误判概率。'
        : '不是只看单笔大单，而是结合挂单、撤单、吃单与最新成交判断动作真假，适合短线席位与量化监控场景。',
      stateLabel: accessLabel,
      bgStart: '#0B0F16',
      bgEnd: '#1A1610',
      accent: '#20D59E',
      glow: '#F2C760',
      panelTitle: '大额行为上下文',
      subtitle: `追踪 ${runtime.whales}+ 类大额异动，并与订单簿及成交结构同步交叉验证`,
      linePath: 'M192 502 C256 470 320 442 382 414 C428 394 468 368 514 350 C538 340 552 328 546 316',
      footerTitle: '大额挂单、撤单与扫单不再是孤立事件',
      footerText: '在同一张图中交叉对照订单簿、最新成交与分析面板，过滤噪声动作。',
      bannerCta: '大额行为',
      bars: [48, 76, 88, 70, 104, 112, 94],
      chips: ['大额行为', '订单流', '成交明细'],
      cards: [
        { label: '鲸鱼异动', value: `${runtime.whales}`, note: '大额行为' },
        { label: '分析面板', value: '联动', note: '同屏判断' },
        { label: '最新成交', value: '实时', note: '结构确认' }
      ]
    },
    {
      key: 'desk',
      kicker: '机构与团队工作流',
      eyebrow: '团队与机构工作流',
      title: hasFullAccess ? '团队与机构工作流可直接承接' : '从个人盯盘到交易席位共用一套专业中枢',
      lead: hasFullAccess
        ? '当前账户已处于完整访问状态，可继续查看机构方案、团队接入与更深层部署能力。'
        : '公开预览承接认知，账户开通承接试用，机构方案承接交易席位协作、量化团队接入与部署能力。',
      stateLabel: accessLabel,
      bgStart: '#0A0F16',
      bgEnd: '#141A25',
      accent: '#7CC7FF',
      glow: '#20D59E',
      panelTitle: '机构接入架构',
      subtitle: `支持公开预览、账户开通与机构接入三层转化路径`,
      linePath: 'M186 510 C236 486 292 456 352 426 C420 390 472 358 546 322',
      footerTitle: '个人、席位团队与机构客户的接入路径清晰分层',
      footerText: '方案展示、席位开通与终端使用形成连续路径，便于机构客户完成评估、接入与落地。',
      bannerCta: '机构接入',
      bars: [42, 58, 74, 88, 102, 118, 132],
      chips: ['公开预览', '账户开通', '机构接入'],
      cards: [
        { label: '接入场景', value: '3 层', note: '预览到机构' },
        { label: '币种覆盖', value: `${runtime.totalSymbols}+`, note: '统一数据底座' },
        { label: '工作流', value: '闭环', note: '发现到执行' }
      ]
    },
    {
      key: 'alerts',
      kicker: '预警通知体系',
      eyebrow: '预警通知与事件流',
      title: hasFullAccess ? '预警流、信号流与通知流已并到同一层' : '让预警不再只是被动通知',
      lead: hasFullAccess
        ? '完整权限下可直接把预警通知、事件流与实时判断整合到同一工作台。'
        : '将强提醒、异动通知、事件流与最近同类提醒放在统一入口，避免消息碎片化。',
      stateLabel: accessLabel,
      bgStart: '#0C1017',
      bgEnd: '#1C1320',
      accent: '#F97316',
      glow: '#F2C760',
      panelTitle: '预警分发中枢',
      subtitle: `最近 ${runtime.feedCount}+ 条事件持续刷新，承接预警通知与同类提醒联动`,
      linePath: 'M188 502 C248 488 308 454 364 428 C422 402 470 376 522 344 C536 336 544 326 548 316',
      footerTitle: '预警不是单点提示，而是带上下文的连续事件流',
      footerText: '最近同类提醒、实时通知、关注理由与联动标的在同一层解释，减少漏看与误读。',
      bannerCta: '预警联动',
      bars: [38, 64, 82, 96, 90, 118, 136],
      chips: ['预警队列', '同类提醒', '推送策略'],
      cards: [
        { label: '通知流', value: `${runtime.feedCount}+`, note: '连续刷新' },
        { label: '同类提醒', value: '10 条', note: '保留最新' },
        { label: '推送链路', value: '联动', note: '不丢上下文' }
      ]
    },
    {
      key: 'replay',
      kicker: '分析与回放闭环',
      eyebrow: '分析面板与回放闭环',
      title: hasFullAccess ? '分析、执行与回放闭环已经形成' : '把分析面板做成可复盘的交易闭环',
      lead: hasFullAccess
        ? '当前访问层级已足以承接分析面板、执行入口与盘后回放的闭环使用。'
        : '不只给出结论，还把盘口、成交、分析面板与关键时刻回放串成可复盘路径。',
      stateLabel: accessLabel,
      bgStart: '#091118',
      bgEnd: '#111B14',
      accent: '#7DD3FC',
      glow: '#20D59E',
      panelTitle: '复盘决策闭环',
      subtitle: `把分析面板、订单簿、最新成交与回放入口统一到同一张决策屏`,
      linePath: 'M190 508 C240 488 286 460 344 442 C404 424 458 392 512 356 C532 344 544 332 548 322',
      footerTitle: '从盘中判断到盘后复盘，不再断链',
      footerText: '关键时刻回放、分析面板与执行入口在同一套终端语言中协同，形成可复用的交易席位工作流。',
      bannerCta: '复盘闭环',
      bars: [46, 52, 74, 92, 108, 122, 138],
      chips: ['分析面板', '关键回放', '执行链路'],
      cards: [
        { label: '分析面板', value: '固定', note: '同屏判断' },
        { label: '回放入口', value: '闭环', note: '关键时刻' },
        { label: '工作流', value: '复用', note: '盘中到盘后' }
      ]
    }
  ].map(item => ({
    ...item,
    primaryCta: primaryBannerCta.label,
    primaryAction: primaryBannerCta.action,
    secondaryCta: secondaryBannerCta.label,
    secondaryAction: secondaryBannerCta.action,
    art: buildHomeBannerSvg(item)
  }));
}

function renderHomeBannerNav(slides, currentIndex) {
  return slides.map((item, index) => `
    <button class="home-banner-dot ${index === currentIndex ? 'act' : ''}" type="button" onclick="selectHomeBanner(${index})" aria-label="${escapePortalHtml(item.eyebrow)}">
      <span></span>
    </button>
  `).join('');
}

function renderHomeBannerTrack(slides, currentIndex) {
  return `
    <div class="home-banner-track" style="transform:translateX(-${currentIndex * 100}%);">
      ${slides.map(item => `
        <article class="home-banner-slide">
          <div class="home-banner-copy">
            <div class="home-banner-kicker">${escapePortalHtml(item.eyebrow)}</div>
            <div class="home-banner-state">${escapePortalHtml(item.stateLabel || '公开预览')}</div>
            <div class="home-banner-title">${escapePortalHtml(item.title)}</div>
            <div class="home-banner-lead">${escapePortalHtml(item.lead)}</div>
            <div class="home-banner-actions">
              <button class="portal-btn primary" type="button" onclick="${item.primaryAction}">${escapePortalHtml(item.primaryCta)}</button>
              <button class="portal-btn secondary" type="button" onclick="${item.secondaryAction}">${escapePortalHtml(item.secondaryCta)}</button>
            </div>
          </div>
          <div class="home-banner-art">
            <img src="${item.art}" alt="${escapePortalHtml(item.title)}">
          </div>
        </article>
      `).join('')}
    </div>
  `;
}

function shiftHomeBanner(delta) {
  const slides = homeBannerSlides(portalRuntimeMetrics());
  if (!slides.length) return;
  const total = slides.length;
  const next = (Number(window.__bbHomeBannerIndex || 0) + Number(delta || 0) + total) % total;
  window.__bbHomeBannerIndex = next;
  refreshHomePortalLive();
  startHomePortalMotion();
}

function bindHomeBannerInteractions() {
  const stage = document.getElementById('home-banner-stage');
  if (!stage || stage.dataset.dragReady === '1') return;
  stage.dataset.dragReady = '1';
  stage.style.touchAction = 'pan-y';

  const drag = { pointerId: null, startX: 0, currentX: 0, active: false };

  const resetTrack = () => {
    stage.classList.remove('is-dragging');
    const track = stage.querySelector('.home-banner-track');
    if (track) {
      track.style.transition = '';
      track.style.transform = '';
    }
  };

  const onFinish = pointerId => {
    if (drag.pointerId === null) return;
    if (pointerId !== undefined && pointerId !== drag.pointerId) return;
    const delta = drag.currentX - drag.startX;
    const shouldShift = Math.abs(delta) > 56;
    try { stage.releasePointerCapture?.(drag.pointerId); } catch (_) {}
    drag.pointerId = null;
    drag.active = false;
    if (shouldShift) {
      shiftHomeBanner(delta > 0 ? -1 : 1);
      return;
    }
    resetTrack();
    refreshHomePortalLive();
  };

  stage.addEventListener('pointerdown', ev => {
    if (ev.target.closest('button')) return;
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;
    drag.pointerId = ev.pointerId;
    drag.startX = ev.clientX;
    drag.currentX = ev.clientX;
    drag.active = true;
    stage.classList.add('is-dragging');
    try { stage.setPointerCapture?.(ev.pointerId); } catch (_) {}
  });

  stage.addEventListener('pointermove', ev => {
    if (!drag.active || drag.pointerId !== ev.pointerId) return;
    drag.currentX = ev.clientX;
    const delta = drag.currentX - drag.startX;
    if (Math.abs(delta) < 6) return;
    const track = stage.querySelector('.home-banner-track');
    if (!track) return;
    track.style.transition = 'none';
    track.style.transform = `translateX(calc(-${Number(window.__bbHomeBannerIndex || 0) * 100}% + ${delta}px))`;
    ev.preventDefault();
  });

  stage.addEventListener('pointerup', ev => onFinish(ev.pointerId));
  stage.addEventListener('pointercancel', ev => onFinish(ev.pointerId));
  stage.addEventListener('pointerleave', ev => {
    if (ev.pointerType === 'mouse') onFinish(ev.pointerId);
  });
}

function refreshHomePortalLive() {
  if (S.site?.page !== 'home') return;
  const portal = document.getElementById('portal-shell');
  if (!portal || !portal.classList.contains('is-active')) return;

  const runtime = portalRuntimeMetrics();
  const pulse = homeSpotlightItems();
  const currentIndex = Number(window.__bbHomeSpotlightIndex || 0) % Math.max(pulse.length, 1);
  const bannerSlides = homeBannerSlides(runtime);
  const bannerIndex = Number(window.__bbHomeBannerIndex || 0) % Math.max(bannerSlides.length, 1);
  const current = pulse[currentIndex] || pulse[0] || PORTAL_FALLBACK_PULSE[0];
  const sideItems = pulse.filter((_, index)=>index!==currentIndex).slice(0, 3);
  const feedItems = (S.feed || []).slice(0, 8);
  const metrics = [
    { label: '监控币种池', value: `${runtime.totalSymbols}+`, note: '统一接入盘口、成交与异常信号' },
    { label: '强提醒候选', value: runtime.strongSignals, note: '盘中高优先级关注标的' },
    { label: '鲸鱼异动', value: runtime.whales, note: '大额挂单、撤单、吃单轨迹' },
    { label: '实时事件流', value: `${runtime.feedCount}+`, note: '信号、预警、回放入口统一沉淀' }
  ];

  pulsePortalNode('home-screen-badge', runtime.accessLabel);
  pulsePortalNode('home-panel-symbol', current.symbol || 'BTCUSDT');
  pulsePortalNode('home-panel-score', String(current.score || 92));
  pulsePortalNode('home-panel-text', current.tag || '强势拉升');

  metrics.forEach((item, index) => {
    pulsePortalNode(`home-hero-metric-${index}`, item.value);
  });

  setPortalHtml('home-story-metrics', renderHomeMetricCards(metrics));
  setPortalHtml('home-spot-nav', renderHomeSpotlightNav(pulse, currentIndex));
  setPortalHtml('home-live-tape', renderHomeLiveTape(feedItems));
  setPortalHtml('home-banner-stage', renderHomeBannerTrack(bannerSlides, bannerIndex));
  setPortalHtml('home-banner-nav', renderHomeBannerNav(bannerSlides, bannerIndex));
  bindHomeBannerInteractions();

  setPortalHtml('home-screen-side', sideItems.map(item=>`
    <div class="home-screen-panel mini">
      <div class="home-mini-row">
        <span>${escapePortalHtml(item.symbol)}</span>
        <b>${escapePortalHtml(String(item.score))}</b>
      </div>
      <div class="home-mini-row sub">
        <span>${escapePortalHtml(item.tag)}</span>
        <span>${escapePortalHtml(item.change)}</span>
      </div>
    </div>
  `).join(''));
}

function stopHomePortalMotion() {
  if (window.__bbHomeSpotlightTimer) {
    clearInterval(window.__bbHomeSpotlightTimer);
    window.__bbHomeSpotlightTimer = null;
  }
  if (window.__bbHomeBannerTimer) {
    clearInterval(window.__bbHomeBannerTimer);
    window.__bbHomeBannerTimer = null;
  }
}

function startHomePortalMotion() {
  stopHomePortalMotion();
  if (S.site?.page !== 'home') return;
  window.__bbHomeSpotlightTimer = setInterval(() => {
    const items = homeSpotlightItems();
    if (!items.length) return;
    window.__bbHomeSpotlightIndex = (Number(window.__bbHomeSpotlightIndex || 0) + 1) % items.length;
    refreshHomePortalLive();
  }, HOME_HERO_ROTATE_MS);
  window.__bbHomeBannerTimer = setInterval(() => {
    const slides = homeBannerSlides(portalRuntimeMetrics());
    if (!slides.length) return;
    window.__bbHomeBannerIndex = (Number(window.__bbHomeBannerIndex || 0) + 1) % slides.length;
    refreshHomePortalLive();
  }, HOME_BANNER_ROTATE_MS);
}

function selectHomeSpotlight(index) {
  window.__bbHomeSpotlightIndex = Math.max(0, Number(index) || 0);
  refreshHomePortalLive();
  startHomePortalMotion();
}

function selectHomeBanner(index) {
  window.__bbHomeBannerIndex = Math.max(0, Number(index) || 0);
  refreshHomePortalLive();
  startHomePortalMotion();
}

function renderHomePortalPage() {
  const runtime = portalRuntimeMetrics();
  const isAuthenticated = !!S.auth?.user;
  const hasFullAccess = !!S.access?.full_access;
  const bannerSlides = homeBannerSlides(runtime);
  const bannerIndex = Number(window.__bbHomeBannerIndex || 0) % Math.max(bannerSlides.length, 1);
  const pulse = homeSpotlightItems();
  const spotlightIndex = Number(window.__bbHomeSpotlightIndex || 0) % Math.max(pulse.length, 1);
  const spotlight = pulse[spotlightIndex] || pulse[0] || PORTAL_FALLBACK_PULSE[0];
  const sideItems = pulse.filter((_, index)=>index!==spotlightIndex).slice(0, 3);
  const metrics = [
    { label: '监控币种池', value: `${runtime.totalSymbols}+`, note: '统一接入盘口、成交与异常信号' },
    { label: '强提醒候选', value: runtime.strongSignals, note: '盘中高优先级关注标的' },
    { label: '鲸鱼异动', value: runtime.whales, note: '大额挂单、撤单、吃单轨迹' },
    { label: '实时事件流', value: `${runtime.feedCount}+`, note: '信号、预警、回放入口统一沉淀' }
  ];
  const trustItems = [
    { label: '覆盖场景', value: '个人 / 团队 / 机构', note: '从个人交易员到团队席位，再到机构接入统一承接。' },
    { label: '决策链路', value: `${Math.max(Number(runtime.strongSignals) || 0, 12)}+`, note: '强提醒、盘口、预警、成交与回放联动形成完整决策链路。' },
    { label: '监控标的', value: `${runtime.totalSymbols}+`, note: '公开预览与账户开通共用同一套标的池与实时刷新体系。' },
    { label: '事件处理', value: `${runtime.feedCount}+`, note: '盘中信号、异动、成交变化与通知流持续刷新。' }
  ];
  const feedItems = (S.feed || []).slice(0, 8);
  const capabilities = [
    {
      title: '实时信号中枢',
      body: '把拉盘、砸盘、盘口失衡、主动买卖量差和异常结构聚合成统一信号墙，优先级清晰，适合盯盘与直播。'
    },
    {
      title: '订单簿与成交联动',
      body: '同一屏内同步查看深度、最新成交、价格节奏和中枢分析，避免在多个页面来回切换导致执行变慢。'
    },
    {
      title: '鲸鱼与预警体系',
      body: '重点追踪大额挂单、撤单、扫单和连续异动，给交易员明确的关注理由，而不是只有涨跌幅。'
    },
    {
      title: '交易与回放闭环',
      body: '从实时信号到模拟下单，再到关键时刻回放复盘，形成可复用的交易工作流。'
    }
  ];
  const stages = [
    { step: '01', title: '发现机会', text: '先用综合排序、强提醒和鲸鱼异动筛出真正值得盯的币种。' },
    { step: '02', title: '确认结构', text: '再看盘口深度、成交节奏、买卖量差和分析面板，过滤掉假动作。' },
    { step: '03', title: '执行与复盘', text: '在同一控制台完成下单、观察、预警和复盘，不切屏，不断链。' }
  ];
  return `
    <section class="portal-page home-page">
      <section class="home-hero home-reveal is-visible">
        ${renderHomeHeroBackdrop()}
        <div class="home-hero-copy">
          <div class="home-hero-kicker">BB-Market / 交易席位与量化团队专业终端</div>
          <h1 class="home-hero-title">面向交易席位、量化团队与机构桌面的实时决策终端。</h1>
          <p class="home-hero-lead">
            围绕交易席位、量化团队与机构桌面的实际使用场景，整合实时信号、订单簿、最新成交、预警通知与执行入口，
            形成覆盖盘前筛选、盘中跟踪与盘后复盘的统一终端能力。
          </p>
          <div class="home-hero-tags">
            <span>实时信号</span>
            <span>预警通知</span>
            <span>鲸鱼追踪</span>
            <span>订单簿分析</span>
            <span>模拟交易</span>
            <span>关键时刻回放</span>
          </div>
          <div class="home-hero-actions">
            <button class="portal-btn primary" type="button" onclick="switchSitePage('ai')">进入 AI 盯盘</button>
            <button class="portal-btn secondary" type="button" onclick="openAuthModal('login')">登录席位账户</button>
            <button class="portal-btn secondary" type="button" onclick="switchSitePage('vip')">申请机构方案</button>
          </div>
          <div class="home-hero-manifesto">
            <div class="home-hero-manifesto-head">
              <span class="home-hero-manifesto-kicker">系统定位</span>
              <strong>为交易席位、量化团队与机构客户提供同一套实时决策终端，覆盖机会识别、结构确认、预警接收与执行承接。</strong>
            </div>
            <div class="home-hero-manifesto-grid">
              <div class="home-hero-manifesto-item">
                <b>席位决策</b>
                <span>将实时信号、鲸鱼轨迹与盘口异动汇总到统一入口，提升交易席位盘中决策效率。</span>
              </div>
              <div class="home-hero-manifesto-item">
                <b>量化监控</b>
                <span>订单簿、最新成交、分析面板与预警通知联动展示，适合量化团队做连续监控与因子验证。</span>
              </div>
              <div class="home-hero-manifesto-item">
                <b>机构交付</b>
                <span>从公开预览、账户开通到机构接入逐层承接，统一扩展标的权限、实时推送与部署能力。</span>
              </div>
            </div>
          </div>
          <div class="home-hero-foot">
            ${metrics.map(item=>`
              <div class="home-hero-foot-item">
                <b id="home-hero-metric-${metrics.indexOf(item)}">${escapePortalHtml(item.value)}</b>
                <span>${escapePortalHtml(item.label)}</span>
              </div>
            `).join('')}
          </div>
        </div>
        <div class="home-hero-visual">
          <div class="home-hero-form-shell">
            ${
              isAuthenticated
                ? `
                  <div class="home-hero-form-card access">
                    <div class="home-hero-form-kicker">账户访问状态</div>
                    <div class="home-hero-form-title">${hasFullAccess ? '当前席位已开通全量市场访问权限' : '当前席位已登录，可继续升级到机构级权限'}</div>
                    <div class="home-hero-form-subtitle">
                      ${hasFullAccess
                        ? `当前权限状态为 ${escapePortalHtml(runtime.accessLabel)}，已开通 ${escapePortalHtml(runtime.totalSymbols)} / ${escapePortalHtml(runtime.totalSymbols)} 个可见标的。`
                        : `当前权限状态为 ${escapePortalHtml(runtime.accessLabel)}，已开通 ${escapePortalHtml(runtime.visibleSymbols)} / ${escapePortalHtml(runtime.totalSymbols)} 个可见标的，可继续升级机构级访问范围。`
                      }
                    </div>
                    <div class="home-hero-form-metrics">
                      <div class="home-hero-form-metric">
                        <span>访问等级</span>
                        <b>${escapePortalHtml(runtime.accessLabel)}</b>
                      </div>
                      <div class="home-hero-form-metric">
                        <span>当前用户</span>
                        <b>${escapePortalHtml(runtime.userLabel)}</b>
                      </div>
                      <div class="home-hero-form-metric">
                        <span>可见市场</span>
                        <b>${escapePortalHtml(runtime.visibleSymbols)} / ${escapePortalHtml(runtime.totalSymbols)}</b>
                      </div>
                    </div>
                    <div class="home-hero-form-actions">
                      <button class="portal-btn primary" type="button" onclick="switchSitePage('ai')">进入专业终端</button>
                      <button class="portal-btn secondary" type="button" onclick="switchSitePage('vip')">${hasFullAccess ? '查看机构方案' : '升级机构权限'}</button>
                    </div>
                  </div>
                `
                : `
                  <form class="home-hero-form-card" onsubmit="submitHeroTrial(event)">
                    <div class="home-hero-form-kicker">试用席位开通</div>
                    <div class="home-hero-form-title">快速开通试用席位，体验 AI 盯盘专业终端。</div>
                    <div class="home-hero-form-subtitle">适合交易席位、量化团队与机构客户先行验证系统能力，后续可按权限范围扩展标的、推送与服务能力。</div>
                    <label class="home-hero-form-field">
                      <span>用户名</span>
                      <input id="hero-trial-username" autocomplete="username" placeholder="trader01">
                    </label>
                    <label class="home-hero-form-field">
                      <span>显示名称</span>
                      <input id="hero-trial-display-name" autocomplete="nickname" placeholder="交易员 A">
                    </label>
                    <label class="home-hero-form-field">
                      <span>登录密码</span>
                      <input id="hero-trial-password" type="password" autocomplete="new-password" placeholder="至少 6 位">
                    </label>
                    <div class="home-hero-form-actions">
                      <button class="portal-btn primary" type="submit">申请试用席位</button>
                      <button class="portal-btn secondary" type="button" onclick="openHeroLogin()">登录已有席位</button>
                    </div>
                    <div class="home-hero-form-note">
                      <span>公开预览</span>
                      <span>席位开通后扩展标的</span>
                      <span>机构方案可按需接入</span>
                    </div>
                    <div class="home-hero-form-msg" id="home-hero-form-msg"></div>
                  </form>
                `
            }
          </div>
          <div class="home-screen">
            <div class="home-screen-top">
              <div class="home-screen-title">AI 盯盘驾驶舱</div>
              <div class="home-screen-badge" id="home-screen-badge">${escapePortalHtml(runtime.accessLabel)}</div>
            </div>
            <div class="home-spot-nav" id="home-spot-nav">
              ${renderHomeSpotlightNav(pulse, spotlightIndex)}
            </div>
            <div class="home-screen-main">
              <div class="home-screen-panel spotlight">
                <div class="home-panel-kicker">盘中焦点</div>
                <div class="home-panel-symbol" id="home-panel-symbol">${escapePortalHtml(spotlight.symbol || 'BTCUSDT')}</div>
                <div class="home-panel-score" id="home-panel-score">${escapePortalHtml(String(spotlight.score || 92))}</div>
                <div class="home-panel-text" id="home-panel-text">${escapePortalHtml(spotlight.tag || '强势拉升')}</div>
              </div>
              <div class="home-screen-side" id="home-screen-side">
                ${sideItems.map(item=>`
                  <div class="home-screen-panel mini">
                    <div class="home-mini-row">
                      <span>${escapePortalHtml(item.symbol)}</span>
                      <b>${escapePortalHtml(String(item.score))}</b>
                    </div>
                    <div class="home-mini-row sub">
                      <span>${escapePortalHtml(item.tag)}</span>
                      <span>${escapePortalHtml(item.change)}</span>
                    </div>
                  </div>
                `).join('')}
              </div>
            </div>
            <div class="home-screen-strip">
              <span>订单簿</span>
              <span>最新成交</span>
              <span>分析面板</span>
              <span>预警通知</span>
            </div>
            <div class="home-live-tape" id="home-live-tape">
              ${renderHomeLiveTape(feedItems)}
            </div>
          </div>
        </div>
        <div class="home-proof-strip">
          <div class="home-proof-head">
            <span class="home-proof-kicker">能力证明</span>
            <strong>面向交易席位、量化团队与机构客户的统一专业终端，而不是割裂的单点功能页面。</strong>
          </div>
          <div class="home-proof-grid">
            ${trustItems.map(item=>`
              <div class="home-proof-item">
                <span>${escapePortalHtml(item.label)}</span>
                <b>${escapePortalHtml(item.value)}</b>
                <small>${escapePortalHtml(item.note)}</small>
              </div>
            `).join('')}
          </div>
        </div>
      </section>

      <section class="home-banner home-reveal" data-reveal-delay="0.02">
        <div class="home-section-head compact">
          <div>
            <div class="home-section-kicker">核心场景</div>
            <div class="home-section-title">围绕交易席位、量化团队与机构客户组织的五类核心能力</div>
          </div>
          <div class="home-banner-controls">
            <button class="home-banner-arrow" type="button" onclick="shiftHomeBanner(-1)" aria-label="上一张">
              <span>&lsaquo;</span>
            </button>
            <div class="home-banner-nav" id="home-banner-nav">
              ${renderHomeBannerNav(bannerSlides, bannerIndex)}
            </div>
            <button class="home-banner-arrow" type="button" onclick="shiftHomeBanner(1)" aria-label="下一张">
              <span>&rsaquo;</span>
            </button>
          </div>
        </div>
        <div class="home-banner-stage" id="home-banner-stage">
          ${renderHomeBannerTrack(bannerSlides, bannerIndex)}
        </div>
      </section>

      <section class="home-partners home-reveal" data-reveal-delay="0.04">
        <div class="home-section-head">
          <div class="home-section-kicker">连接能力</div>
          <div class="home-section-title">面向交易席位、量化团队与机构客户的一体化连接中枢</div>
        </div>
        ${renderHomePartnerRail()}
      </section>

      <section class="home-story home-reveal" data-reveal-delay="0.08">
        <div class="home-story-main">
          <div class="home-section-kicker">系统说明</div>
          <div class="home-story-title">不是单一行情终端，而是覆盖盘前筛选、盘中决策与盘后复盘的专业 SaaS 终端。</div>
          <div class="home-story-text">
            BB-Market 以 SaaS 方式交付实时信号、订单簿、最新成交、分析面板、预警通知与执行入口，统一服务于交易席位、量化团队与机构客户。
            产品表达聚焦能力边界、适用场景与服务路径，终端入口则面向实时监控、交易判断与执行协同。
          </div>
        </div>
        <div class="home-story-grid" id="home-story-metrics">
          ${renderHomeMetricCards(metrics)}
        </div>
      </section>

      ${renderHomeArchitecture(runtime, pulse)}

      <section class="home-capabilities home-reveal" data-reveal-delay="0.12">
        <div class="home-section-head">
          <div class="home-section-kicker">核心能力</div>
          <div class="home-section-title">可直接服务于交易席位、量化团队与机构协同的核心系统能力</div>
        </div>
        <div class="home-cap-grid">
          ${capabilities.map((item, index)=>`
            <article class="home-cap-card home-reveal" data-reveal-delay="${(0.16 + index * 0.06).toFixed(2)}">
              <div class="home-cap-title">${escapePortalHtml(item.title)}</div>
              <div class="home-cap-body">${escapePortalHtml(item.body)}</div>
            </article>
          `).join('')}
        </div>
      </section>

      <section class="home-flow home-reveal" data-reveal-delay="0.16">
        <div class="home-flow-board">
          <div class="home-section-kicker">工作流</div>
          <div class="home-section-title">一套适用于席位交易、量化监控与盘后复盘的统一工作流</div>
          <div class="home-flow-list">
            ${stages.map((item, index)=>`
              <article class="home-flow-item home-reveal" data-reveal-delay="${(0.18 + index * 0.06).toFixed(2)}">
                <div class="home-flow-step">${escapePortalHtml(item.step)}</div>
                <div class="home-flow-title">${escapePortalHtml(item.title)}</div>
                <div class="home-flow-text">${escapePortalHtml(item.text)}</div>
              </article>
            `).join('')}
          </div>
        </div>
        <div class="home-flow-side">
          <div class="home-side-card emphasis">
            <div class="home-side-kicker">产品定位</div>
            <div class="home-side-title">围绕机构客户评估、接入与使用形成清晰的产品路径。</div>
            <div class="home-side-text">页面内容聚焦方案能力、适用对象与服务方式，终端入口聚焦实时决策与交易执行，前台表达与后台使用边界更清楚。</div>
          </div>
          <div class="home-side-card">
            <div class="home-side-kicker">适用对象</div>
            <div class="home-side-list">
              <span>交易席位</span>
              <span>量化团队</span>
              <span>研究支持</span>
              <span>机构桌面</span>
            </div>
          </div>
        </div>
      </section>
      ${renderPortalFooter('home')}
    </section>
  `;
}

function renderPortalPage(page) {
  const spec = PORTAL_PAGES[page] || PORTAL_PAGES.about;
  const runtime = portalRuntimeMetrics();
  return `
    <section class="portal-page">
      <div class="portal-hero">
        <div class="portal-kicker">${escapePortalHtml(spec.kicker)}</div>
        <div class="portal-title">${escapePortalHtml(spec.title)}</div>
        <div class="portal-lead">${escapePortalHtml(spec.lead)}</div>
        <div class="portal-highlights">${(spec.highlights || []).map(item=>`<span>${escapePortalHtml(item)}</span>`).join('')}</div>
        ${renderPortalActions(page)}
      </div>
      ${renderPortalMetrics(spec.metrics(runtime))}
      <div class="portal-layout">
        <div class="portal-main">
          ${(spec.sections || []).map(renderPortalSection).join('')}
        </div>
        ${renderPortalSidebar(runtime)}
      </div>
      ${renderPortalFooter(page)}
    </section>
  `;
}

function syncPortalNav(page, trigger = null) {
  document.querySelectorAll('.site-nav-link[data-page], .site-nav-subbtn, .portal-footer-link').forEach(btn => {
    btn.classList.toggle('act', btn.dataset.page === page);
  });
  document.querySelectorAll('.site-nav-menu').forEach(menu => {
    const active = !!menu.querySelector(`.site-nav-subbtn[data-page="${page}"]`);
    menu.classList.toggle('act', active);
  });
  if (trigger && trigger.dataset?.page) {
    trigger.classList.add('act');
  }
}

function mountSitePage(page) {
  const portal = document.getElementById('portal-shell');
  const dashboard = document.getElementById('dashboard-shell');
  if (!portal || !dashboard) return;

  if (page === 'ai') {
    stopHomePortalMotion();
    portal.classList.remove('is-active');
    portal.innerHTML = '';
    dashboard.classList.remove('is-hidden');
    return;
  }

  portal.innerHTML = page === 'home' ? renderHomePortalPage() : renderPortalPage(page);
  portal.classList.add('is-active');
  dashboard.classList.add('is-hidden');
  if (page === 'home') {
    refreshHomePortalLive();
    startHomePortalMotion();
    observeHomeReveal();
  } else {
    stopHomePortalMotion();
  }
}

function normalizedSitePage(page) {
  if (!page) return 'home';
  if (page === 'announcement') return 'announcements';
  return PORTAL_PAGES[page] ? page : 'home';
}

function switchSitePage(page, trigger = null) {
  const nextPage = normalizedSitePage(page);
  S.site.page = nextPage;
  syncPortalNav(nextPage, trigger);
  mountSitePage(nextPage);
  if(typeof updateDocumentTitle==='function'){
    const current=S.sel?getSymbolState(S.sel):null;
    updateDocumentTitle(S.sel,current?fP(sv(S.sel,'mid')):'--',current?.change_24h_pct??null);
  }
  const hash = nextPage === 'home' ? '' : `#${nextPage}`;
  if (location.hash !== hash) {
    history.replaceState(null, '', `${location.pathname}${location.search}${hash}`);
  }
}

function refreshSitePage() {
  if (S.site?.page === 'home') {
    refreshHomePortalLive();
    return;
  }
  switchSitePage(S.site?.page || normalizedSitePage(location.hash.replace('#', '')));
}

function setNavMenuOpen(menu, open) {
  if (!(menu instanceof HTMLElement)) return;
  menu.classList.toggle('open', !!open);
  const trigger = menu.querySelector('.site-nav-trigger');
  if (trigger instanceof HTMLElement) {
    trigger.setAttribute('aria-expanded', open ? 'true' : 'false');
  }
}

function closeNavMenus(except = null) {
  document.querySelectorAll('.site-nav-menu').forEach(menu => {
    if (menu === except) return;
    setNavMenuOpen(menu, false);
  });
}

function bindNavMenuBehavior() {
  document.querySelectorAll('.site-nav-menu').forEach(menu => {
    if (menu.dataset.bound === '1') return;
    menu.dataset.bound = '1';
    const trigger = menu.querySelector('.site-nav-trigger');

    if (trigger instanceof HTMLElement) {
      trigger.setAttribute('aria-haspopup', 'true');
      trigger.setAttribute('aria-expanded', 'false');
    }

    menu.addEventListener('mouseenter', () => {
      closeNavMenus(menu);
      setNavMenuOpen(menu, true);
    });

    menu.addEventListener('mouseleave', () => {
      setNavMenuOpen(menu, false);
    });

    if (trigger instanceof HTMLElement) {
      trigger.addEventListener('click', ev => {
        ev.preventDefault();
        const nextOpen = !menu.classList.contains('open');
        closeNavMenus(nextOpen ? menu : null);
        setNavMenuOpen(menu, nextOpen);
      });
    }

    menu.querySelectorAll('.site-nav-subbtn').forEach(btn => {
      btn.addEventListener('click', () => {
        setNavMenuOpen(menu, false);
        if (btn instanceof HTMLElement) btn.blur();
        if (trigger instanceof HTMLElement) trigger.blur();
      });
    });
  });

  if (!document.body.dataset.navWatchBound) {
    document.body.dataset.navWatchBound = '1';
    document.addEventListener('pointerdown', ev => {
      const target = ev.target;
      if (!(target instanceof Element)) return;
      if (target.closest('.site-nav-menu')) return;
      closeNavMenus();
    });
    document.addEventListener('focusin', ev => {
      const target = ev.target;
      if (!(target instanceof Element)) return;
      if (target.closest('.site-nav-menu')) return;
      closeNavMenus();
    });
  }
}

function initPortal() {
  const initialPage = normalizedSitePage(location.hash.replace('#', ''));
  S.site.page = initialPage;
  syncPortalNav(initialPage);
  mountSitePage(initialPage);
  bindNavMenuBehavior();
  window.addEventListener('hashchange', () => {
    const page = normalizedSitePage(location.hash.replace('#', ''));
    if (page !== S.site.page) {
      switchSitePage(page);
    }
  });
}

window.initPortal = initPortal;
window.refreshHomePortalLive = refreshHomePortalLive;
window.selectHomeSpotlight = selectHomeSpotlight;
window.selectHomeBanner = selectHomeBanner;
window.shiftHomeBanner = shiftHomeBanner;
window.refreshSitePage = refreshSitePage;
window.switchSitePage = switchSitePage;
