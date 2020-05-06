# saizefs
ファイルシステムで直に味わうサイゼリヤ.

## saizefsファイルサーバーを実行
```
./download_saizeriya_db.sh  # 'saizeriya.db'をダウンロード. ありがとう、@marushosummers.
cargo run --release 'tcp!0.0.0.0!12345' # 'protocol!listen_addr!port'
```

## saizefsをマウント
```
sudo mount -t 9p -o version=9p2000.L,trans=tcp,port=12345 127.0.0.1 /path/to/mountdir
```

## ファイルシステムで味わうサイゼリヤ
```
$ cd /path/to/mountdir/ && ls
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アーリオ・オーリオ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アーリオ・オーリオ（Wサイズ）
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アラビアータ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アラビアータ（Wサイズ）
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アンチョビのピザ（ルーコラ葉入り）
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 アンチョビのピザ（ルーコラ葉入り） + トッピングチーズ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 イカの墨入りスパゲッティ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 イカの墨入りスパゲッティ（Wサイズ）
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 イタリアンハンバーグ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 イタリアンプリン
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 いちごソースのパンナコッタ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 いろどり野菜のミラノ風ドリア
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 いろどり野菜のミラノ風ドリア + トッピングチーズ
dr-xr-xr-x 1 saize saize 0 Jan  1 17:29 エスカルゴのオーブン焼き
...

$ cd たらこクリームのピザ/
$ ls
calorie  category  id  price  salt  type
$ cat calorie
615
$ cat category
meal
$ cat price
399
$ cat type
pizza
```


## 少し説明
`saizefs` は9Pプロトコル(正確には9P2000.L)でLinuxカーネルと通信して, ユーザー空間で動くネットワーク透過なファイルシステムサーバーとして動作します.
ユーザーは普通に `mount` コマンドでローカルにマウントできてファイルシステムとして動作します.

ちなみに `protocol` として `tcp` だけでなく `unix` でUNIXドメインソケットで通信もできます.詳しくは[ここ](https://github.com/pfpacket/rust-9p)を確認してください.
