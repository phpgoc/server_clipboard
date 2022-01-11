
## 获取

``
    cargo install server_clipboard
``

## https证书

- 测试环境自己生成
- 没有https无法获得剪贴板权限，火狐ie不支持剪贴板
- If you want to generate your own cert/private key file, then run:


```bash
    mkcert test.xx
```

`mkcert`: https://github.com/FiloSottile/mkcert


## 运行
``
    server_clipboard -c cert.pem -k key.pem
``

## 存入key, value<br>
``
    curl -X POST -d "$value" $host/$key
``

## 获取k 
``
    curl $host/$key?quiet
``
## 参数，
### POST和 websocket的第一进入房间的人有效
- times optional int  可以获取的次数默认1 
- minutes optional int 保存的分钟 默认1分钟    
- private optional any 是否在首页列表显示，websocket都是public的
- quiet option any get页独有，禁用websocket，为了curl

## demo 

``
curl -X POST -d "abcdefg" "localhost:7259/abc?times=2&private=a" 
``
## get页面支持websocket,鼠标单击就会write 或 read 剪贴板

