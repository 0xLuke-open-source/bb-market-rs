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
    visibleSymbols: S.access?.visible_symbols || Math.min(syms.length || 24, 8),
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
    actions.push({ label: '返回首页盯盘', action: "switchSitePage('home')", kind: 'primary' });
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

  if (page === 'home') {
    portal.classList.remove('is-active');
    portal.innerHTML = '';
    dashboard.classList.remove('is-hidden');
    return;
  }

  portal.innerHTML = renderPortalPage(page);
  portal.classList.add('is-active');
  dashboard.classList.add('is-hidden');
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
window.refreshSitePage = refreshSitePage;
window.switchSitePage = switchSitePage;
