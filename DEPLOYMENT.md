# Deployment

Live: https://stellar-shield-frontend.vercel.app

The three contracts are deployed on Stellar testnet. The frontend reads them
directly over Soroban RPC from the browser, so the dashboard works with no
backend running; BACKEND_URL is only needed for passkey registration.

| contract | testnet address |
| --- | --- |
| guard | `CDHAOCU3SQ5FJK3K2TT76T74V7SYEQUN6GU2EXX3A7BZR7XYVPYUMB5T` |
| registry | `CAWXVQKWWGP62YNYQIQ6AYAPP7EYNPCUT4MB3LDSCEVWLSQRGYYP45AK` |
| auth | `CDRIB5FDB34NORXUP7SIVU3OF6BKKFBRVJLS23QFNWPTMG7QH3DYGMPL` |

Network: Test SDF Network ; September 2015
RPC: https://soroban-testnet.stellar.org
