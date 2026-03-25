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
    lead: '把直播盯盘、专属席位、策略陪跑、企业部署、API白名单和专属客服打包成完整服务，而不是只卖一个页面账号。',
    highlights: ['机构席位', '专属群与客服', '策略共研', '私有部署支持'],
    metrics: runtime => [
      { label: '开放套餐', value: runtime.planCount, note: '当前接口返回可订阅计划数' },
      { label: '企业客户线索', value: '128', note: '近30日登记咨询' },
      { label: '续费率', value: '78.4%', note: '季度 VIP 续费' },
      { label: '平均响应', value: '7 分钟', note: '专属客服工作时段' }
    ],
    sections: [
      {
        type: 'cards',
        title: '服务层级',
        desc: '把不同客群分清楚，页面上直接讲权益与适用对象。',
        items: [
          { title: '个人 Pro', body: '适合高频观察盘面和多策略切换的个人交易员，重点是完整监控池与实时推送。', meta: '面向活跃个人用户' },
          { title: 'Desk 团队版', body: '适合 3-20 人研究 / 交易小组，支持账号席位、内部同步、权限控制与管理后台。', meta: '面向团队' },
          { title: '机构私有版', body: '支持隔离部署、独立风控、企业白名单、数据保留策略与定制接口。', meta: '面向机构与项目方' }
        ]
      },
      {
        type: 'table',
        title: '权益矩阵',
        desc: '直接把用户会问的差异列清楚。',
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
        desc: '页面要能承接销售线索，而不是只有一堆形容词。',
        items: [
          { title: '量化团队', text: '需要更稳定的信号产出、内部多席位复盘、对接自己的风控和成交系统。' },
          { title: '媒体 / 社群', text: '需要对外展示的市场异动、公告、热币观察和直播控台。' },
          { title: '项目方 / 做市商', text: '需要观察市场深度、异常波动和舆情联动，及时处理流动性问题。' }
        ]
      }
    ]
  },
  ads: {
    kicker: 'Ad Network',
    title: '广告解决方案',
    lead: '广告页不是简单放价格表，而是清楚告诉投放方能买到什么位置、什么流量、什么数据回收能力。',
    highlights: ['首页黄金位', '内容赞助', '社群联动', '效果回传'],
    metrics: () => [
      { label: '月均曝光', value: '240万+', note: '站内页面 + 社群联动' },
      { label: '平均 CTR', value: '3.8%', note: '首页核心资源位' },
      { label: '合作品牌', value: '56', note: '近 12 个月已合作项目' },
      { label: '最快上线', value: '24 小时', note: '素材齐全后' }
    ],
    sections: [
      {
        type: 'cards',
        title: '可售资源位',
        desc: '给广告主的页面必须把库存讲明白。',
        items: [
          { title: '首页导航推荐', body: '适合交易所活动、工具产品、投研栏目和平台品牌展示。', meta: '品牌曝光' },
          { title: 'AI盯盘专题赞助', body: '在高关注度内容页中挂出专题卡、权益引导和转化入口。', meta: '精准触达' },
          { title: '社区联合活动', body: '结合 AMA、抽奖、任务和内容共创，放大单次投放效果。', meta: '活动转化' }
        ]
      },
      {
        type: 'table',
        title: '投放套餐示例',
        desc: '这里先放一版运营型示例数据，后面你可以直接替换成真实报价。',
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
        desc: '提前回答预算、审核和追踪相关问题。',
        items: [
          { q: '支持哪些素材形式？', a: '支持横幅、卡片、长图、视频封面、落地页跳转和外部活动报名页。' },
          { q: '投放前是否审核项目？', a: '会。高风险金融承诺、传销型活动、虚假空投和违规引流不接。' },
          { q: '是否提供数据回传？', a: '可提供曝光、点击、跳转、报名和活动参与等维度的投放复盘。' }
        ]
      }
    ]
  },
  feedback: {
    kicker: 'Feedback Loop',
    title: '产品反馈与建议',
    lead: '把反馈页做成真正可运营的需求入口，让用户知道该提什么、多久响应、会不会进入排期。',
    highlights: ['需求收集', 'Bug 反馈', '优先级回执', '路线图沟通'],
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
        desc: '用户越清楚怎么提，团队收到的需求质量越高。',
        items: [
          { title: '交易工作流痛点', text: '例如你在哪个步骤最容易丢信号、误判、漏单或无法复盘。' },
          { title: '想新增的数据维度', text: '例如你需要资金费率、新闻事件、链上地址、社媒热度、更多盘口因子。' },
          { title: '具体页面 Bug', text: '请附上路径、时间、浏览器、账户状态和复现步骤，修复效率会高很多。' }
        ]
      },
      {
        type: 'cards',
        title: '反馈处理流程',
        desc: '页面上直接告诉用户不是石沉大海。',
        items: [
          { title: '1. 收集与归类', body: '按照 Bug、体验优化、新功能、商业合作四类进入不同队列。', meta: 'T+0' },
          { title: '2. 评估优先级', body: '综合影响面、实现成本、商业价值和安全风险分配优先级。', meta: 'T+1' },
          { title: '3. 回执与排期', body: '对重要需求给出是否采纳、预计版本和替代方案。', meta: 'T+2' }
        ]
      },
      {
        type: 'faq',
        title: '提交建议前先看',
        desc: '减少重复反馈。',
        items: [
          { q: '哪里提功能需求最快？', a: '优先通过站内表单或社群管理员提交，附带具体场景与截图会更快进入评估。' },
          { q: '怎么确认需求有没有被接收？', a: '页面会明确展示回执 SLA，重点需求会收到专门回复或进入公告页更新。' },
          { q: '可不可以直接约演示？', a: '可以，机构或高价值用户建议走 VIP 服务页对接。' }
        ]
      }
    ]
  },
  rebate: {
    kicker: 'Rebate Program',
    title: '超级返佣',
    lead: '返佣页需要把规则、比例、结算方式和适合人群讲清楚，避免用户只看到“高返佣”却不知道怎么参与。',
    highlights: ['高比例返佣', '邀请链路可视化', '月度结算', '专属客服支持'],
    metrics: () => [
      { label: '合作交易所', value: '9', note: '支持返佣跟踪' },
      { label: '最高返佣', value: '55%', note: '视渠道等级而定' },
      { label: '月发放佣金', value: '128,000 USDT', note: '示例运营数据' },
      { label: '活跃推广者', value: '1,460', note: '近30日' }
    ],
    sections: [
      {
        type: 'table',
        title: '返佣等级示例',
        desc: '这里放一版直观的层级表。',
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
        desc: '不仅给比例，还要给场景。',
        items: [
          { title: '内容导流', body: '适合做短视频、直播、图文的交易类 KOL，把专属开户链接和活动页组合起来。', meta: '适合内容型推广' },
          { title: '社群裂变', body: '适合社区主理人和招商团队，通过群任务、打卡、教程和晒单提升转化。', meta: '适合社群运营' },
          { title: '机构合作', body: '适合有大量活跃交易用户的团队，用 API 或后台看板追踪实际贡献。', meta: '适合渠道合作' }
        ]
      }
    ]
  },
  invite: {
    kicker: 'Referral Growth',
    title: '邀请奖励',
    lead: '邀请页主打拉新转化，应该突出奖励门槛、达成路径和实时榜单，而不是把返佣和邀请混在一起。',
    highlights: ['邀请注册奖励', '订阅转化奖励', '排行榜激励', '活动任务'],
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
        desc: '把不同动作的奖励拆开。',
        items: [
          { title: '邀请注册', body: '被邀请人成功注册并完成首次登录，即可获得基础积分或现金券。', meta: '拉新奖励' },
          { title: '订阅转化', body: '被邀请人完成订阅后，邀请人获得更高等级现金奖励。', meta: '核心奖励' },
          { title: '排行榜加成', body: '每周按有效邀请人数和订阅额进行排行，榜单前列额外获得奖金池。', meta: '活动激励' }
        ]
      },
      {
        type: 'list',
        title: '适合谁做邀请',
        desc: '页面要告诉用户“我能不能做”。',
        items: [
          { title: '活跃老用户', text: '熟悉产品、有真实使用体验，转化率通常更高。' },
          { title: '内容创作者', text: '可把教程、复盘、盯盘视频和注册链接组合传播。' },
          { title: '社群主理人', text: '适合在群内做体验营、训练营和打卡活动。' }
        ]
      }
    ]
  },
  plaza: {
    kicker: 'Plaza',
    title: '广场',
    lead: '广场页用于承接用户内容、短观点、热帖、精选信号、达人观察和热门话题，做成一个“边看盘边刷”的内容场。',
    highlights: ['热帖榜', '精选观点', '短线观察', '达人关注'],
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
        desc: '用卡片模拟广场内容流。',
        items: [
          { title: 'BTC 是否进入加速段？', body: '多位交易员围绕盘口主动买量、ETF 资金回流和关键阻力位展开讨论。', meta: '2.1k 浏览' },
          { title: 'SOL 巨鲸回补后还能追吗', body: '围绕鲸鱼进场信号、近三小时成交结构和回撤风险做实盘拆解。', meta: '1.6k 浏览' },
          { title: '异常断层与假突破案例库', body: '社区整理了近两周最典型的盘口断层假突破案例，适合复盘学习。', meta: '980 浏览' }
        ]
      },
      {
        type: 'list',
        title: '广场内容板块',
        desc: '先做出结构，后续可以对接真实接口。',
        items: [
          { title: '精选信号', text: '自动把高质量币种信号推送成讨论主题，提高内容和行情联动。' },
          { title: '实盘复盘', text: '鼓励用户发布自己的进出场逻辑、失误与修正，形成方法库。' },
          { title: '热点话题', text: '围绕大盘、政策、山寨轮动、链上事件组织主题内容。' }
        ]
      }
    ]
  },
  blog: {
    kicker: 'Insights',
    title: '博客',
    lead: '博客页主要承担内容沉淀、SEO 和品牌建立，适合发方法论、产品更新、案例复盘和行业观察。',
    highlights: ['方法论文章', '功能更新', '市场洞察', '案例复盘'],
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
        desc: '给博客页先放一批看起来像真的内容卡。',
        items: [
          { title: '如何用盘口失衡识别假突破', body: '从订单簿倾斜、主动买卖量差和大额挂单撤单节奏拆解常见骗线。', meta: '策略方法论' },
          { title: '交易员版 Dashboard 的设计思路', body: '为什么我们把信号墙、市场列表、下单区和告警区做成一体化屏幕。', meta: '产品设计' },
          { title: '鲸鱼进场信号的 5 个误判场景', body: '并不是所有大单都值得跟，重点在于持续性、位置和成交结构。', meta: '案例复盘' }
        ]
      },
      {
        type: 'faq',
        title: '博客运营说明',
        desc: '内容页也要有管理逻辑。',
        items: [
          { q: '文章多久更新一次？', a: '建议每周至少 3 篇，覆盖产品、策略、行业和活动内容。' },
          { q: '是否支持嘉宾投稿？', a: '支持，尤其欢迎真实交易案例、风控经验和深度研究内容。' },
          { q: '是否能跳转到对应功能页？', a: '可以，文章中可直接引导到 AI 盯盘、VIP 服务、社区和活动页。' }
        ]
      }
    ]
  },
  help: {
    kicker: 'Help Center',
    title: '帮助中心',
    lead: '帮助中心应该覆盖新手入门、功能说明、订阅说明、常见异常和账户问题，减少客服压力。',
    highlights: ['新手入门', '账号与订阅', '功能操作', '问题排查'],
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
        desc: '帮助页先把目录结构立起来。',
        items: [
          { title: '快速开始', text: '如何注册、登录、订阅、切换页面、查看信号、进入交易面板。' },
          { title: '权限与套餐', text: '公开模式和订阅模式的区别、套餐到期、退款与续费说明。' },
          { title: '页面问题排查', text: '如果看不到数据、连不上 WebSocket、按钮没反应、图表不显示，先看这里。' }
        ]
      },
      {
        type: 'faq',
        title: '高频问题',
        desc: '帮助中心一定要先顶最常见问题。',
        items: [
          { q: '为什么首页有些币种看不到？', a: '公开模式仅开放部分币种与功能，订阅后解锁完整监控池。' },
          { q: '为什么控制台提示 Origin not allowed？', a: '这通常来自浏览器钱包或第三方扩展注入脚本，不是站点资源 404；页面业务逻辑修好后，这类报错通常可忽略。' },
          { q: '为什么页面样式或脚本 404？', a: '如果遇到 `/static/css/portal.css` 或 `/static/js/app/portal.js` 404，说明站点页资源未发布完整；本次改造已把它们补上。' }
        ]
      }
    ]
  },
  announcements: {
    kicker: 'Announcements',
    title: '公告',
    lead: '公告页用于发布版本上线、服务维护、活动通知、套餐变更和重要风险提示，是站内最硬的信息区。',
    highlights: ['版本更新', '维护通知', '活动上新', '风险提示'],
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
        desc: '直接做一条可读的时间线。',
        items: [
          { time: '03-24 10:00', title: '站点页门户系统上线', text: '新增 AI 盯盘、VIP 服务、广告、帮助中心、公告、新闻中心、社区等导航页。' },
          { time: '03-23 21:30', title: '订阅套餐展示优化', text: '新增套餐权益矩阵、访问态提示和订阅后的解锁文案。' },
          { time: '03-22 14:00', title: '异常监控能力升级', text: '新增盘口断层、异常刷量与大额撤单识别信号。' }
        ]
      }
    ]
  },
  news: {
    kicker: 'Newsroom',
    title: '新闻中心',
    lead: '新闻中心偏资讯聚合和专题运营，跟公告不同，它更像“市场发生了什么，以及这对用户意味着什么”。',
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
        desc: '示例新闻卡片。',
        items: [
          { title: 'BTC 再次测试关键压力位，AI 盯盘信号同步升温', body: '结合盘口主动买量与鲸鱼行为，平台将其列入首页重点关注列表。', meta: '市场焦点' },
          { title: '山寨轮动加剧，如何从信号墙筛选高质量标的', body: '从“只看涨幅”切换到“看成交结构 + 异常 + 鲸鱼”的筛选方式。', meta: '专题解析' },
          { title: '交易员为什么需要一体化盯盘界面', body: '当信号、盘口、交易和告警分散在多个页面时，执行效率会明显下降。', meta: '深度观察' }
        ]
      }
    ]
  },
  community: {
    kicker: 'Community',
    title: '社区',
    lead: '社区页承接官方群、区域群、主题群、活动群和合作伙伴社群，是长期留存与品牌扩散的关键入口。',
    highlights: ['官方群矩阵', '主题社群', '活动运营', '合作伙伴社群'],
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
        desc: '先给出结构和定位。',
        items: [
          { title: '官方公告群', body: '同步版本更新、重要活动、系统维护和福利通知。', meta: '信息同步' },
          { title: '交易讨论群', body: '围绕热点币种、AI 信号、复盘和策略展开交流。', meta: '核心用户' },
          { title: '合作伙伴群', body: '适合渠道、KOL、项目方、代理和企业客户对接合作。', meta: '商务拓展' }
        ]
      },
      {
        type: 'faq',
        title: '社区运营说明',
        desc: '把规则讲清楚，减少后续治理成本。',
        items: [
          { q: '社区是否允许发广告？', a: '普通讨论群不允许乱发广告，合作需求请走广告页或商务渠道。' },
          { q: '是否有地区或语言分群？', a: '可以逐步扩展到中文主群、英文群、区域群和主题群。' },
          { q: '是否有官方直播或活动？', a: '建议和公告页、广场页、博客页联动，形成周期性内容节奏。' }
        ]
      }
    ]
  },
  agreement: {
    kicker: 'Legal',
    title: '服务协议',
    lead: '协议页不必做成纯法务文书堆叠，也可以用分节展示，让用户至少知道最关键的边界条件。',
    highlights: ['服务边界', '用户义务', '风险声明', '责任限制'],
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
        desc: '先做成用户能读完的版本。',
        items: [
          { title: '信息服务属性', text: '平台提供数据展示、分析信号、内容服务和相关工具，不构成任何收益承诺。' },
          { title: '账户安全责任', text: '用户需要妥善保管账户凭据，不得共享、转售、盗用或进行破坏性访问。' },
          { title: '风险自担', text: '所有交易行为由用户自行决策并承担风险，平台不对行情波动和外部平台风险负责。' },
          { title: '违规处理', text: '对于刷号、滥用、违法内容和攻击行为，平台保留限制、终止和追责权利。' }
        ]
      }
    ]
  },
  privacy: {
    kicker: 'Privacy',
    title: '隐私说明',
    lead: '隐私页要回答三件事：收什么、为什么收、怎么保护，以及用户能做什么。',
    highlights: ['收集范围', '使用目的', '安全措施', '用户权利'],
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
        desc: '先把要点说人话。',
        items: [
          { title: '我们收集什么', text: '包括账户信息、订阅记录、页面使用日志和用于保障稳定性的必要技术信息。' },
          { title: '为什么收集', text: '用于提供功能、保障安全、改进产品、处理工单、完成交易与订阅相关服务。' },
          { title: '如何保护', text: '采取权限控制、日志审计、分级存储和必要的传输保护措施。' },
          { title: '你能做什么', text: '可申请查看、更正、注销或删除部分个人信息，法律另有规定的除外。' }
        ]
      }
    ]
  },
  about: {
    kicker: 'About BB-Market',
    title: '关于我们',
    lead: '关于页用于讲清楚团队在做什么、为什么做、面向谁，以及希望形成怎样的产品路线和品牌认知。',
    highlights: ['交易员视角', '数据驱动', '内容与工具一体化', '面向长期产品化'],
    metrics: () => [
      { label: '产品方向', value: '交易工具 + 内容平台', note: '双轮驱动' },
      { label: '覆盖场景', value: '盯盘 / 交易 / 内容 / 商务', note: '多页面协同' },
      { label: '迭代节奏', value: '周更', note: '建议运营节奏' },
      { label: '当前版本', value: 'Portal + Dashboard', note: '首页与门户统一' }
    ],
    sections: [
      {
        type: 'cards',
        title: '我们在做什么',
        desc: '把品牌表达和产品结构统一起来。',
        items: [
          { title: '做交易员真正愿意开的屏', body: '不是堆指标，而是把最关键的市场判断、操作入口和复盘能力集中到一个地方。', meta: '核心产品观' },
          { title: '做内容和工具一体的平台', body: '用户不只看信号，还能看资讯、活动、教程、社区和商务合作入口。', meta: '平台化方向' },
          { title: '做能持续运营的站点', body: '每个页面都不仅仅是占位，而是能承接流量、转化、服务和增长的业务模块。', meta: '商业化方向' }
        ]
      },
      {
        type: 'list',
        title: '下一阶段重点',
        desc: '给页面一点 roadmap 感。',
        items: [
          { title: '接入真实内容源', text: '把博客、新闻、公告、广场和社区逐步接到真实后台。' },
          { title: '补充表单与提交能力', text: '反馈页、广告页、VIP 页和合作页后续应补充正式提交入口。' },
          { title: '继续打磨首页控制台', text: '优化移动端、响应式和权限态下的展示差异。' }
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
        <div class="home-section-title">把行情输入、信号引擎、预警路由和交易桌面，做成一条可执行的数据链路</div>
      </div>
      <div class="home-arch-layout">
        <div class="home-arch-board">
          <div class="home-arch-line" aria-hidden="true"></div>
          <article class="home-arch-node home-reveal is-node-accent" data-reveal-delay="0.14">
            <div class="home-arch-node-kicker">输入层</div>
            <div class="home-arch-node-title">Market Feed</div>
            <div class="home-arch-node-text">盘口、成交、K线、异动事件统一进入同一条实时链路。</div>
          </article>
          <article class="home-arch-node home-reveal" data-reveal-delay="0.22">
            <div class="home-arch-node-kicker">计算层</div>
            <div class="home-arch-node-title">Signal Engine</div>
            <div class="home-arch-node-text">OBI、OFI、主动买卖量差、鲸鱼轨迹和异常结构在这里收敛成评分。</div>
          </article>
          <article class="home-arch-node home-reveal is-node-primary" data-reveal-delay="0.30">
            <div class="home-arch-node-kicker">决策层</div>
            <div class="home-arch-node-title">AI Radar</div>
            <div class="home-arch-node-text">将复杂市场结构翻译成交易员能直接用的强提醒、关注理由和优先级。</div>
          </article>
          <article class="home-arch-node home-reveal" data-reveal-delay="0.38">
            <div class="home-arch-node-kicker">分发层</div>
            <div class="home-arch-node-title">Alert Router</div>
            <div class="home-arch-node-text">实时信号、预警通知、回放链路和订阅权限在这里完成路由。</div>
          </article>
          <article class="home-arch-node home-reveal is-node-accent-alt" data-reveal-delay="0.46">
            <div class="home-arch-node-kicker">执行层</div>
            <div class="home-arch-node-title">Trader Desk</div>
            <div class="home-arch-node-text">最终落到 AI 盯盘控制台、交易面板和复盘流程，形成闭环。</div>
          </article>
        </div>
        <div class="home-arch-side home-reveal" data-reveal-delay="0.18">
          <div class="home-arch-side-card">
            <div class="home-side-kicker">当前焦点链路</div>
            <div class="home-side-title">${escapePortalHtml(primary.symbol)}</div>
            <div class="home-side-text">${escapePortalHtml(primary.tag)}，系统会把该币种直接推进到盯盘驾驶舱与预警流中。</div>
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

function buildHomeBannerSvg(spec) {
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
    <g transform="translate(${332 + index * 104},64)">
      <rect width="92" height="28" rx="14" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.12)"/>
      <text x="46" y="18" text-anchor="middle" fill="#F3F6FB" font-size="10" font-weight="700">${item}</text>
    </g>
  `).join('');
  const cards = (spec.cards || []).slice(0, 3).map((item, index) => `
    <g transform="translate(${324 + index * 108},118)">
      <rect width="96" height="86" rx="18" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.10)"/>
      <text x="16" y="24" fill="#9FB0C7" font-size="9" font-weight="700">${item.label}</text>
      <text x="16" y="50" fill="#FFFFFF" font-size="22" font-weight="800">${item.value}
        <animateTransform attributeName="transform" type="translate" values="0 0;0 -4;0 0" dur="${1.7 + index * 0.2}s" repeatCount="indefinite"/>
        <animate attributeName="opacity" values="0.82;1;0.82" dur="${1.7 + index * 0.2}s" repeatCount="indefinite"/>
      </text>
      <text x="16" y="68" fill="${spec.accent}" font-size="9" font-weight="700">${item.note}</text>
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
      <text x="116" y="224" fill="#F5F7FB" font-size="74" font-weight="900">${spec.title}</text>
      <text x="116" y="274" fill="#C8D1DC" font-size="28">${spec.subtitle}</text>
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
    </svg>`;
  return svgToDataUri(svg);
}

function homeBannerSlides(runtime) {
  const isAuthenticated = !!S.auth?.user;
  const hasFullAccess = !!S.access?.full_access;
  const accessLabel = hasFullAccess ? '已订阅解锁' : (isAuthenticated ? '已登录未订阅' : '公开预览');
  const primaryBannerCta = hasFullAccess
    ? { label: '进入实时控制台', action: "switchSitePage('ai')" }
    : (isAuthenticated
      ? { label: '升级完整权限', action: "switchSitePage('vip')" }
      : { label: '创建试用账户', action: "openAuthModal('register')" });
  const secondaryBannerCta = hasFullAccess
    ? { label: '查看系统方案', action: "switchSitePage('vip')" }
    : (isAuthenticated
      ? { label: '进入 AI 盯盘', action: "switchSitePage('ai')" }
      : { label: '已有账号，立即登录', action: "openAuthModal('login')" });
  return [
    {
      key: 'signals',
      kicker: 'REAL-TIME SIGNALS',
      eyebrow: '实时信号中枢',
      title: hasFullAccess ? '完整实时中枢已经就绪' : (isAuthenticated ? '已登录，下一步解锁完整信号覆盖' : '让强提醒先于波动出现'),
      lead: hasFullAccess
        ? '当前账户已具备完整访问能力，可以直接进入 AI 盯盘控制台。'
        : (isAuthenticated
          ? '你已经进入系统访问态，继续升级即可解锁更多币种、完整推送与更深层信号。'
          : '把拉盘、砸盘、成交节奏和盘口异动压缩成交易员可执行的首屏判断。'),
      stateLabel: accessLabel,
      bgStart: '#0A1016',
      bgEnd: '#132231',
      accent: '#F2C760',
      glow: '#1890FF',
      panelTitle: 'Signal Scoreboard',
      subtitle: `监控 ${runtime.totalSymbols}+ 币种，持续刷新 ${runtime.feedCount}+ 条事件流`,
      linePath: 'M194 486 C246 454 302 438 356 418 C394 404 438 392 470 374 C500 358 530 340 546 320',
      footerTitle: `实时信号 + 盘口确认 + 执行入口`,
      footerText: '避免在行情页、预警页、成交页之间来回切换，首屏直接给出交易员视角。',
      bannerCta: 'Signal First',
      bars: [58, 82, 96, 72, 114, 94, 126],
      chips: ['Signal Wall', 'Depth Flow', 'Alert Engine'],
      cards: [
        { label: '强提醒候选', value: `${runtime.strongSignals}`, note: '盘中优先级' },
        { label: '事件流', value: `${runtime.feedCount}+`, note: '实时刷新' },
        { label: '响应', value: '秒级', note: '不等全页' }
      ]
    },
    {
      key: 'whales',
      kicker: 'WHALE + ORDERBOOK',
      eyebrow: '鲸鱼与盘口联动',
      title: isAuthenticated ? '把鲸鱼动作和盘口确认合到同一层' : '把大资金轨迹放回盘口上下文',
      lead: isAuthenticated
        ? '登录后继续放大订单簿、最新成交和鲸鱼联动的判断深度，减少只看大单的误判。'
        : '不是只看一笔大单，而是结合挂单、撤单、吃单和最新成交，判断动作真假。',
      stateLabel: accessLabel,
      bgStart: '#0B0F16',
      bgEnd: '#1A1610',
      accent: '#20D59E',
      glow: '#F2C760',
      panelTitle: 'Whale Context Matrix',
      subtitle: `追踪 ${runtime.whales}+ 类鲸鱼异动，叠加订单簿与成交结构同步观察`,
      linePath: 'M192 502 C256 470 320 442 382 414 C428 394 468 368 514 350 C538 340 552 328 546 316',
      footerTitle: '大额挂单、撤单、扫单不再是孤立事件',
      footerText: '同一张图里交叉对照订单簿、最新成交和分析面板，过滤噪声动作。',
      bannerCta: 'Whale Context',
      bars: [48, 76, 88, 70, 104, 112, 94],
      chips: ['Whale Trace', 'Order Flow', 'Trade Tape'],
      cards: [
        { label: '鲸鱼异动', value: `${runtime.whales}`, note: '大额行为' },
        { label: '分析面板', value: '联动', note: '同屏判断' },
        { label: '最新成交', value: '实时', note: '结构确认' }
      ]
    },
    {
      key: 'desk',
      kicker: 'DESK WORKFLOW',
      eyebrow: '团队与机构工作流',
      title: hasFullAccess ? '团队与机构工作流可以直接承接' : '从个人盯盘到团队席位共用一套中枢',
      lead: hasFullAccess
        ? '当前账户已处于完整访问状态，可以继续查看机构方案、团队接入和更深层部署能力。'
        : '公开预览先承接认知，登录后承接试用，机构方案承接团队与部署能力。',
      stateLabel: accessLabel,
      bgStart: '#0A0F16',
      bgEnd: '#141A25',
      accent: '#7CC7FF',
      glow: '#20D59E',
      panelTitle: 'Desk Access Layer',
      subtitle: `支持公开预览、登录解锁与机构接入三层转化路径`,
      linePath: 'M186 510 C236 486 292 456 352 426 C420 390 472 358 546 322',
      footerTitle: '个人 / Desk / 机构 接入语义清楚',
      footerText: '首页负责品牌和转化，AI 页负责实时控制台，路径更清晰，转化更直接。',
      bannerCta: 'Desk Layer',
      bars: [42, 58, 74, 88, 102, 118, 132],
      chips: ['Public Preview', 'Login Unlock', 'Desk Access'],
      cards: [
        { label: '接入场景', value: '3 层', note: '预览到机构' },
        { label: '币种覆盖', value: `${runtime.totalSymbols}+`, note: '统一数据底座' },
        { label: '工作流', value: '闭环', note: '发现到执行' }
      ]
    },
    {
      key: 'alerts',
      kicker: 'ALERT STREAM',
      eyebrow: '预警通知与事件流',
      title: hasFullAccess ? '预警流、信号流和通知流已并到同一层' : '让预警不再只是被动通知',
      lead: hasFullAccess
        ? '完整权限下可以直接把预警通知、事件流和实时判断整合到同一工作台。'
        : '把强提醒、异动通知、事件流和最近同类提醒放在统一入口，避免消息碎片化。',
      stateLabel: accessLabel,
      bgStart: '#0C1017',
      bgEnd: '#1C1320',
      accent: '#F97316',
      glow: '#F2C760',
      panelTitle: 'Alert Stream Router',
      subtitle: `最近 ${runtime.feedCount}+ 条事件持续刷新，承接预警通知与同类提醒联动`,
      linePath: 'M188 502 C248 488 308 454 364 428 C422 402 470 376 522 344 C536 336 544 326 548 316',
      footerTitle: '预警不是单点提示，而是带上下文的连续事件流',
      footerText: '最近同类提醒、实时通知、关注理由与联动币种在同一层解释，减少漏看和误读。',
      bannerCta: 'Alert Flow',
      bars: [38, 64, 82, 96, 90, 118, 136],
      chips: ['Alert Queue', 'Recent Similar', 'Push Routing'],
      cards: [
        { label: '通知流', value: `${runtime.feedCount}+`, note: '连续刷新' },
        { label: '同类提醒', value: '10 条', note: '保留最新' },
        { label: '推送链路', value: '联动', note: '不丢上下文' }
      ]
    },
    {
      key: 'replay',
      kicker: 'ANALYTICS + REPLAY',
      eyebrow: '分析面板与回放闭环',
      title: hasFullAccess ? '分析、执行与回放闭环已经形成' : '把分析面板做成可复盘的交易闭环',
      lead: hasFullAccess
        ? '当前访问层级已经足够承接分析面板、执行入口与盘后回放的闭环使用。'
        : '不只给结论，还把盘口、成交、分析面板和关键时刻回放串成能复盘的路径。',
      stateLabel: accessLabel,
      bgStart: '#091118',
      bgEnd: '#111B14',
      accent: '#7DD3FC',
      glow: '#20D59E',
      panelTitle: 'Replay Decision Stack',
      subtitle: `把分析面板、订单簿、最新成交与回放入口统一到一张决策屏`,
      linePath: 'M190 508 C240 488 286 460 344 442 C404 424 458 392 512 356 C532 344 544 332 548 322',
      footerTitle: '从盘中判断到盘后复盘，不再断链',
      footerText: '关键时刻回放、分析面板和执行入口在同一套语言里工作，形成复用型交易工作流。',
      bannerCta: 'Replay Loop',
      bars: [46, 52, 74, 92, 108, 122, 138],
      chips: ['Analysis Panel', 'Trade Replay', 'Execution Loop'],
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
    { label: '团队接入场景', value: '个人 / Desk / 机构', note: '从个人交易员到团队席位，再到机构接入统一承接。' },
    { label: '策略工作流', value: `${Math.max(Number(runtime.strongSignals) || 0, 12)}+`, note: '强提醒、盘口、预警、成交与回放联动形成完整工作流。' },
    { label: '币种覆盖', value: `${runtime.totalSymbols}+`, note: '公开预览与登录解锁共用同一套币种与实时刷新体系。' },
    { label: '实时事件流', value: `${runtime.feedCount}+`, note: '盘中信号、异动、成交变化与通知流持续刷新。' }
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
          <div class="home-hero-kicker">BB-Market / AI Native Trading Intelligence</div>
          <h1 class="home-hero-title">不是看行情，而是比市场更早一步发现机会。</h1>
          <p class="home-hero-lead">
            把实时信号、鲸鱼轨迹、盘口结构、预警通知和交易执行，
            压缩进一张秒级响应的决策屏。首页讲清系统价值，AI盯盘直接承接实时控制台。
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
            <button class="portal-btn secondary" type="button" onclick="openAuthModal('login')">登录解锁全量币种</button>
            <button class="portal-btn secondary" type="button" onclick="switchSitePage('vip')">申请机构方案</button>
          </div>
          <div class="home-hero-manifesto">
            <div class="home-hero-manifesto-head">
              <span class="home-hero-manifesto-kicker">Brand Manifesto</span>
              <strong>让交易员用一张屏完成发现机会、确认结构、接收预警和执行动作，不再被多个页面拆碎。</strong>
            </div>
            <div class="home-hero-manifesto-grid">
              <div class="home-hero-manifesto-item">
                <b>更早发现</b>
                <span>把实时信号、鲸鱼轨迹和盘口异动放在同一个判断入口，减少错过窗口的概率。</span>
              </div>
              <div class="home-hero-manifesto-item">
                <b>更快确认</b>
                <span>订单簿、最新成交、分析面板和预警通知联动显示，不靠来回切屏确认真假动作。</span>
              </div>
              <div class="home-hero-manifesto-item">
                <b>更好转化</b>
                <span>公开预览先看能力边界，登录后继续放大可见币种、实时推送和机构协作能力。</span>
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
                    <div class="home-hero-form-kicker">Access Console</div>
                    <div class="home-hero-form-title">${hasFullAccess ? '当前账户已解锁完整市场访问' : '当前账户已登录，可继续升级到全量权限'}</div>
                    <div class="home-hero-form-subtitle">
                      ${hasFullAccess
                        ? `当前状态为 ${escapePortalHtml(runtime.accessLabel)}，可见币种 ${escapePortalHtml(runtime.totalSymbols)} / ${escapePortalHtml(runtime.totalSymbols)}。`
                        : `当前状态为 ${escapePortalHtml(runtime.accessLabel)}，可见币种 ${escapePortalHtml(runtime.visibleSymbols)} / ${escapePortalHtml(runtime.totalSymbols)}。`
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
                      <button class="portal-btn primary" type="button" onclick="switchSitePage('ai')">进入实时控制台</button>
                      <button class="portal-btn secondary" type="button" onclick="switchSitePage('vip')">${hasFullAccess ? '查看机构方案' : '升级完整权限'}</button>
                    </div>
                  </div>
                `
                : `
                  <form class="home-hero-form-card" onsubmit="submitHeroTrial(event)">
                    <div class="home-hero-form-kicker">Start Free Preview</div>
                    <div class="home-hero-form-title">30 秒内创建试用账户，直接进入 AI 盯盘。</div>
                    <div class="home-hero-form-subtitle">公开预览可先体验，创建账户后继续解锁更多币种、推送与后续订阅能力。</div>
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
                      <button class="portal-btn primary" type="submit">创建试用账户</button>
                      <button class="portal-btn secondary" type="button" onclick="openHeroLogin()">已有账号，立即登录</button>
                    </div>
                    <div class="home-hero-form-note">
                      <span>公开预览</span>
                      <span>登录解锁更多币种</span>
                      <span>机构方案可扩展</span>
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
            <span class="home-proof-kicker">Trust Layer</span>
            <strong>面向交易员、Desk 团队与机构席位的统一工作台，不是单点功能页面。</strong>
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
            <div class="home-section-kicker">系统 Banner</div>
            <div class="home-section-title">根据系统能力自动生成的品牌轮播图</div>
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
          <div class="home-section-kicker">系统连接能力</div>
          <div class="home-section-title">面向交易团队、内容团队与机构桌面的一体化中枢</div>
        </div>
        ${renderHomePartnerRail()}
      </section>

      <section class="home-story home-reveal" data-reveal-delay="0.08">
        <div class="home-story-main">
          <div class="home-section-kicker">系统介绍</div>
          <div class="home-story-title">不是再做一个行情页，而是把交易员最常切换的四类能力做成一个闭环。</div>
          <div class="home-story-text">
            BB-Market 把实时信号、订单簿、最新成交、分析面板、预警通知和执行入口统一到同一工作台。
            首页先讲清楚系统价值与定位，AI盯盘页则直接承接盘中操作，让新用户看得懂，老用户进来就能用。
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
          <div class="home-section-title">给盯盘、交易、复盘和运营都能直接落地的系统能力</div>
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
          <div class="home-section-kicker">操作路径</div>
          <div class="home-section-title">一套适合盘前筛选、盘中执行、盘后复盘的工作流</div>
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
            <div class="home-side-kicker">当前定位</div>
            <div class="home-side-title">首页展示品牌与系统能力，AI页承接实时控制台。</div>
            <div class="home-side-text">导航语义更清楚，对外展示和内部使用分层，避免“首页像后台，AI页像介绍页”的错位。</div>
          </div>
          <div class="home-side-card">
            <div class="home-side-kicker">适用对象</div>
            <div class="home-side-list">
              <span>短线交易员</span>
              <span>研究团队</span>
              <span>社群直播</span>
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
