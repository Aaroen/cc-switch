# AnyRouter 使用问题解决集合

收集一下 AnyRouter 使用过程中遇到各种问题的解决方法

## 网络问题

1. 又来了 502/520/524 是 自己网络问题！不确定什么情况的 请抓包贴上请求和响应

> 来源：
> 1. https://linux.do/t/topic/1302928/9

### HTTP 520: Origin Error/Error: 520 status code (no body)

1. 环境变量加一条 “CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC”: “1” 如果还不行就 clear 或者启动的时候不 - c 开一个新的会话
2. 试试把～/.claude 下的 statsig 和 session-env 备份到别处去
3. 用了几天感觉 https://pmpjfbhq.cn-nb1.rainapp.top 这个请求地址比较稳可以试试，我之前用其他的也总是 520 这种
4. 520 单纯网络不行
5. "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":true
6. 可能是当前项目上下文太长 新开一个就可以

> 来源：
> 1. https://linux.do/t/topic/1089832/10
> 2. https://linux.do/t/topic/1089832/31
> 3. https://linux.do/t/topic/1089832/62
> 4. https://linux.do/t/topic/1289288/26
> 5. https://linux.do/t/topic/1284272
> 6. https://linux.do/t/topic/1302928/26

### HTTP 524: Origin Time-out

基本上无法干涉 归结为 换个网络再试

> 来源：https://linux.do/t/topic/1289288/43

### 502 status code (no body)

1. 502 自己反代一下 any 的地址
2. 把～/.claude 下的 statsig 和 session-env backup 到别处再试试
3. 使用 cloudflare workers 反向代理可以使用，https://github.com/xixu-me/Xget

> 来源：
> 1. https://linux.do/t/topic/1213340/60
> 2. https://linux.do/t/topic/1213340/127
> 3. https://linux.do/t/topic/1157822/66

### API Error: Cannot read properties of undefined (reading 'map')

开思考就行

> 来源：https://linux.do/t/topic/1223275/4

### API Error: undefined is not an object (evaluating 'H.map')

UA问题

> 来源：QQ 群