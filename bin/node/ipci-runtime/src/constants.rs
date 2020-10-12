///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use hex_literal::hex;
    use node_primitives::{AccountId, Balance};
    use sp_core::ecdsa::Public;
    use sp_runtime::{traits::IdentifyAccount, MultiSigner};
    use sp_std::vec;

    pub const U_MITO: Balance = 1_000_000;
    pub const MITO: Balance = 1_000_000 * U_MITO;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 15 * MITO / 100 + (bytes as Balance) * 6 * MITO / 100
    }

    #[cfg(feature = "std")]
    lazy_static::lazy_static! {
        pub static ref STAKE_HOLDERS: Vec<(AccountId, Balance)> = vec![
            (MultiSigner::from(Public::from_full(&hex!["adf0295af32437b7352e4858dbd872fb7d2e36c370d99a5c8dc0888038d4d0d2657118f34bb2a91707e4152e1e3730fb05d94f1a4ed592c54b9fc579c6b5c73b"][..]).unwrap()).into_account(), 183054 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["776f5f06e1271d6deabb776b93a0062c6fea8afabc4864a5b46a38d5dd33f2df0794118ef774512d125b4b7a5b53e333c6c6d54fe09f432115acaf5e1793decc"][..]).unwrap()).into_account(), 10000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f5776abcb3da2466cb4a549aa74d09dcfc08882f2bb5aba199758e9e71902f1c4a02ad63db33e4837548dec6d2b51e1e19bd46183eba7bdc7350606cb98762c5"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["90275ae00acd3bece6acbaf4d27da09153e9b1ca33c7a3142bdac2b8b2ab3d95c9489f5c717a00999fd2ac64f797fbd518c452b9aa761198445c572a9afb1772"][..]).unwrap()).into_account(), 10000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ef92a940e94cd7b494de2784cdfa2c55c6fd0b7b01e5589c8d7ba303e867e0512ceb5c90c54ac83f983790c0df1336e0f54a461712317c20f23543ba66b6daba"][..]).unwrap()).into_account(), 9816 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["069e1744e4860fec3cb7193a60c6f5049e417179e46469c04c46241456c141154a015f876cf1646ff42c648b79a4ab9363c061f0fc6bb01c497edab55f3038f0"][..]).unwrap()).into_account(), 8334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b1724781416f2991e052e9bb0615e984fb984863793735a0bdb1fdf2864c6ad7f592ab0e9fc36119cd6c55d11d66af76d17bc0cb9bb9e83a9370eeb2013f4c8a"][..]).unwrap()).into_account(), 6667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c3264d6cdba86dc4b8168c6f8a0ace7e8cc4778000809a90855c11d2bbbe70a76736cf51914e59c2840ad9e10421e7366b77e9abd7b09ab0d2f81ba52f66a90d"][..]).unwrap()).into_account(), 6667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["3d368c62ec53f5b48f48faa07f20a2c61100a167011e3e5b7307b62ce359242ed45a03aa2ba04143a435598fa9919a18dc8f7582b457330d1235ca5f9a1c50ce"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9e535c6910b9915a7596e129c7591d0aa29bb18100295e0a5b793107a19ede6e4d5c0a4bc75ecc035942cfe898fc2d0df9ea2f1473d62dfbae01c4baa32b5dea"][..]).unwrap()).into_account(), 1208 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1acb3bf5c37187fd37abfda16efb25e309e1a7e0e131fbb4d0005c8db04953cec772eb0bcc378adcf6995e960956dd4cbf0ccd38f7d1f4215fafd119ee4ab2b6"][..]).unwrap()).into_account(), 23000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0c611c8db9229c12c2982d2dd0324dd3f3bb029d71da852789c68b617a78389af51d0a74d3a1a92de40c6c3204641966009c951c976f98fe91bef2e73cf0020f"][..]).unwrap()).into_account(), 264 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a19bc5d298e95ea4f0a3dae5d11514ddf05febe87e17ce15a37793bf67beffec0abbf06176a52b5e0d2ad74b546e3ac8c7dfbbef64626305dca13a2b17f11b09"][..]).unwrap()).into_account(), 11434 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4c92421bf6e871a49be4f8bddb2493010280dcc87a34a07ef4b09ca2635b2c7b7f2fdfb46b35961445a47d831d36ac2c30c823182a4f3e8f691de928bad30c1b"][..]).unwrap()).into_account(), 36327 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ecacd597c798bc10813da350d9e85b01bf0f96b03481a678e0da3c663ce5a39bd533d18bdb98c61b122f72be0a8020b0402de0b3861301d298d7c100ce24daab"][..]).unwrap()).into_account(), 1334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9090dc592bbbe8e7b3f9d6689b23a67419a7f2e3ebc73b51873e8a04c155ce8643884b90e65b8e8520a0fee376ed0c0579201ad7e3a70847fe80df518fcfa107"][..]).unwrap()).into_account(), 84 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5a3648b3e89cac46f50dc3cc74e9c337963c5ac002ebd8ddf15402fce69c5d96cf6f1085e55571b0543312d11527aa70eced161488382556a98d7d6ed5b5c94f"][..]).unwrap()).into_account(), 512 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f0b4a651f87f35e14aaef2a96ce97c2d1ab7fc242459fc3f0d1bba8544b8266ddec8f84929041ea7ac22b272e28502fd51b80fb329b699dcb7cdf3779094df87"][..]).unwrap()).into_account(), 6667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1ad0c295726904827513fa8c5c7f4a0c4a203fce1120b96db7b3d7b5d50f93b3b12d2b3b3b0d4f224275b4dfb05082a8a7f196da141726e6cd9cb857964832c1"][..]).unwrap()).into_account(), 3620 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2ea9b6b503aca26afdfc92390ef8abcdda1e4f0ce9b1e73c75bff946a49a4cd52610356de8f38b3913d3541a40fb83139ee1662d2f65626713e01554fb96f6ae"][..]).unwrap()).into_account(), 33333 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["612838e8741bde0c0edab262f192395f9f4c1a7c2ccdad20e540de15e9f2a5980f739c190b1a5ca9f157af5c30784ba05598d6c1639a60f56999fe07c2fbdc8d"][..]).unwrap()).into_account(), 15667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a48c6cbe89ce7ff324799acc5702b2ddcaa3df92600e08d9c00f2ba582f1374beb221a8c46787c381386086970cdb1dcad15e91e8c7b3cb39ad42b8db064fa53"][..]).unwrap()).into_account(), 16667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7b10b54b3b8691a10d77e058221d9cbf4173b233473c950a755556dadb60a6621e0ee5397d3ede79ecd381ea606b4dbd6f4f33414d7dad32c64ca20f851b7447"][..]).unwrap()).into_account(), 3340 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4d072f2029909ebebe1307091307f3e98bff0fafdc6bd63b83fb5d39b2fbb934e1b4ca8127765e98833dd5a9e288e583d050fc77f4bb78bc7b21e900fa8eb64f"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0177929080d90d32098703596cad9a8fa389199643bb7c3de9debcb579cb5898059ef47247f75f5dd97ab563c05c6d7d3dd730505d28e5ba9e8034968b834652"][..]).unwrap()).into_account(), 2000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fcf544f381de32d50733627f270ec9a10e62d648d9db9044031d2d851720d8e76de7a38187f3bdbf098e70c1b12cf81f98e38e72843ff3b40a53ed69af0d9608"][..]).unwrap()).into_account(), 21334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5375b00ad75ba3da86db9ef4cc951bfb5f38aaea8cbc91629ec4a4ab145e6eafc5de307a2de0ccc26a9209d1f22ad367949b4c5ff8cf24a479e926d5dbb04e58"][..]).unwrap()).into_account(), 637 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["bed4e9e2127dfb0e5dbf1c5630e6c4ef104bbf14cf17ff750f8ab20a3e9a87cdb24fb965f4a3218647d5a0dbbb6d64a6b8ced487b2e8467178d68c7fe25b4b9c"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6473c7e12824d98f7ad21c44e35ef77f944ac7980592977e6d2d9cbfe8dd218f6afb08b55fcd76ee9d5d25c45012dd737e40684100be2259401ad0f6cb5adf38"][..]).unwrap()).into_account(), 314 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9b11afd1ce16445d0d37d9ffc229066eb9c86469264ebce6dfdf5df84846b671147ab1a0dc08ada29dfc782a33be454fc13133902e008298288031a471739c3d"][..]).unwrap()).into_account(), 1667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e446caa5ccc987767ddb70826adb8ae7bd004d4ff468a4ed338077e7595e0082b845d1431985a803ce16803ba6908a325872cf6e8ad9d4333104b392becb52ca"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["27c03a520c19571cdbe4388fb2ca1a616d2f14dca9c1225e6c90d50d80c4d09f5f4489c0fee3ec5abc5139914a76f14791f8449079cb63321c71ddf67df07909"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["549158c51757a4e197cd1883b6ab4478801e3240724364c7afd1fa11a4860e1f3b3d5325fe4e0aefea4920f8112eb61d03db91c6c5b4fe983942645765951369"][..]).unwrap()).into_account(), 15000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c45a634a45cd19c2774b6699dca60f370e69494532ac47dfbd98a7ccd6f44ba78e0dc2166bc66836e79694909bd83fe98ce750859c9f8ca86aad217300e2dff5"][..]).unwrap()).into_account(), 6667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fff9fa0b3cd9ba58193b4e8be8a1654d1715977fadc633a672da7ae6e0ae27745d8b113d8c3fde1388906c6960164bc117f969b434866e0206eec61493c41b5d"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e4dbe6f98b7bed6ef86dd4f808562d2005b227638cc8b67a1a349f7dc1711b07a0ef59827e77ba18fb4aac8aa7ba83d79537c8a290b1fe495651d627012cfb5f"][..]).unwrap()).into_account(), 3683 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2e5c6f9751f9da691bf630f1919b25778f5f4a703183038c0da88ef138bdf469cac4c5ff6dac10b51a5b678c32ae39fd58066028e8ebf170f9108d55862fbcb8"][..]).unwrap()).into_account(), 633 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["cb7a367212275d374767c0aac76685b29487eefc72dd3108f388bddcacdfed49341d6c8c9c1e9c0db7063a23b3ebe825f219cc253422bfafd8ce5c94beafd03a"][..]).unwrap()).into_account(), 31667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["955361d4e2754c5570c8fe8078b11bc549e3d11da308b1c8569552ac6eddb4aa7deb1e9b1431b0b274b0e65d383f2b746ddd2a8966b2ae71eebac8fae73a42dd"][..]).unwrap()).into_account(), 10000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["3d73db137ee7b1e76b22a58c517f0262583bfb648830815da59620746075e233bba596a234fa5f1e2be13c05e66987819fd4c6a449ada6712b91e314ca7d5160"][..]).unwrap()).into_account(), 16667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["774ed670ab451bcf82d953664f1fdf0b0961deaefd93f78cc76667ead19b10311c1cd34083eb31ae97e6c92fab987c1b01cbc812c580b5a9c36b82bf81ade327"][..]).unwrap()).into_account(), 1134 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["413e35bcddfb34991d6db4fb8b024caf68759495b8af849690f34216c9d93307da9a6104bc1e457474715fb0f6883b7ef492d5df5cdac658c2565e3d02d3d652"][..]).unwrap()).into_account(), 1334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ba8fd80107bea5fb491a9100b06fa6a134881b5d6e348faa3c24291807ef1e655a327db35198250d3901ce7b14b52600caab6328c5672ffa8af28f50902e51e4"][..]).unwrap()).into_account(), 667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4c671c26e9587c85cffb5b21c2e54653868d6877e01202e0f5b0c82ac087287ff05ecef3aaa6a319ae428d3f378f5e09147d243552c705e1af1ed9dd67b2a8e2"][..]).unwrap()).into_account(), 1667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f6c4f530802b1a1dc15aa627a2ee1dd1dd132b775b88e6c7e7bf6fbbb464d7e21f327083db5396e7f979fe996897d5692d8a343e251da171a41334d142593d06"][..]).unwrap()).into_account(), 634 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["894c4d9e5c3f8e87e6f067504acd2e589232a700c14b12861401a1e78f4ecdc168a6ff02e7f7918cc45c21cc88f7370ff004f6ac63c06fccd56862ec7fa894e8"][..]).unwrap()).into_account(), 2667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["650cc2688668cf1659de9286a082fe1a83f2e534c84c624a82595b91aee8a35bac4265ab1f165bbfdd746ac773ce7f83a8b5b137e2f70362fcfbbe10ae83e968"][..]).unwrap()).into_account(), 23346 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d55ffda4a7dae498b6d4bf69b42a37d050636011e70e90aae5b28feeaec0cecced1592507f8d492978229258c900fb8a243941676bc211bf819dea5e666a760e"][..]).unwrap()).into_account(), 10000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ece1d4871642376633c0428f8e988276a7036052b72c50d61e48d83547735ceaf498b694f1b337d3515d316f919e63c4f801d8d89f7ef1fcaf2fae7461027450"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a885069e325c49b49008fb6f34f7b179efb6b556f03c82152da2a8a7830af8ceddec34c4c6cc53903e69658479425c288e370f8dac7501667b1254dffbd38fe6"][..]).unwrap()).into_account(), 13267 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ad4f1f07704440e0c655fda9745ac13275a42717345c1466ee6fd488be0e7bd58eb202087c6d6d66d0209270d6ab2cd99dd784b80ddf3f90b539acb106d51813"][..]).unwrap()).into_account(), 334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4d3680eab64aebcc573baa8fe5350f0d8ffd1081a6f1924627590c3d785baeaef00ff2123c2f264be143335504aa358a1cfb884321d719af5859fb89c7dc5181"][..]).unwrap()).into_account(), 834 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["749bbcd76aabc8d53d87e385357b3ac2015e57b355421d576110d06c8c3ac8565204bab23a4257cf701016931a143be29ae0c6e8cdf11808fdb468620ce1ef5b"][..]).unwrap()).into_account(), 3167 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["af6047015118373c5a1e8cd9ddf131f17ec3f5969591fef33774ca8816e858e6e8060f0dff5caee145163343d770f51d6d9aeb5541e2100f41341f579bd5c484"][..]).unwrap()).into_account(), 6667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0bd50a5c715c2c3ab4c6f026c1c416c470950aa241da2481075c2a76f6d93015b5505a0882422ad7925e20862388009c5e3aae0f8f598a614b6386e66baa5f1b"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b3e2c5dc950f5d3f5d328c4fba9d8cf39023544fe4f2e21f13b2dfc5c182132fa8cc1722b4f67c98816b1829f05ac23c2787411672c7cde10d0984fa3e183e74"][..]).unwrap()).into_account(), 334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["977981698c11bd5a8a59e147ebee2678e51bb702c51f1a413d5feb5bb014af60a05eea9a438da7388ef92a1b3efb22389b99e90c741d904edf9f9f31d27f4b1f"][..]).unwrap()).into_account(), 187 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6fbea73029b27d418ea90324f8819c1ca9b47ab2a1550625e2074be16f9a8617b309f0504e8eed27a29fdc0bd5893be6ce9f15409db54cfff4f44c0b272c27f8"][..]).unwrap()).into_account(), 34 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4b1dbce9eb78330be4136b2ef2609cff95a76a6bdd1f67b8e8fbd75d533dce0a532bf38a0a857fac85ab5370afacd652e5ab767427b8fc5bd0f11372376e53c0"][..]).unwrap()).into_account(), 3537 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["398222a57d2fc369170575e728ca356acac4406288eb555a0598950ac24ad8603186f1a09fe2e5c4af1229aef6e2af7bb2257f19f58d26bcee12c2db631de1b9"][..]).unwrap()).into_account(), 500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4c5904514893966c716d53e28bab69a94dc1300f533f8f22a0c27a09a8258c217deec74f429fbddd0004d87932b0aa9748d73d618c76925513ff92ce3ae60c54"][..]).unwrap()).into_account(), 100 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["651289ddda04ed8f08711c7cfae6dafb8c2f46ca73009c9be3981492051b28aa0b41bc0373b05a2b230cfbdb676eadca5a5da191e369624541b7472159beb9f3"][..]).unwrap()).into_account(), 1667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a7d214a4a7ebf03ec73003f33be31c129e7e023e551c5dad3dfc8bfd0f03b15a58a2f75369d693681dff417661ef4981fa936bd58df214820c6407147e0ec653"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ba532df9e0ebccdf8964d1f5b163eeb117f4f34b4d26927213f0254694fdb2cdc5851cf061cdc0d1aa1237274bcdc2067ad5d44eaa039eebb45d6f1f142af5aa"][..]).unwrap()).into_account(), 334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a24625ce90152d0a1b4bd6aa66a273b0684d69bac4d338c64e1bf5479965f0bec261cf29ecfc0b727d847a5967c211bfa253b9c70b1b7feb2b419d95401b27ca"][..]).unwrap()).into_account(), 43323 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["adba0e0a181cdd816804acce15b14e4528717e8c545c7082b0193477aaaa5de28286ff618fe4deb617d6574c31678eab75863d2416ce1f0b40c2ff46c3a0dd9c"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4e695668b7682965ad909995e8309dfd1a348bd489b644ac8822042eb9632b485b29fdb93879d5c85c11a3ff5c72e1f065884a0e6c1b8b3bcf762ef35ae21788"][..]).unwrap()).into_account(), 524 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["20449af394327a21b8add9df8dba02b4f7343b827ae974c289f935ee18951f148814885ae47d0f52d979c18f06db2a755050bacdc7a3a126eb4305af4c5454db"][..]).unwrap()).into_account(), 117 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e118330a8a494aa29dbf18f6607d061f7958b57c632d029e881bdc74225cd09d7823ac9b0cc2ca2200645c4d4e07e11409bc7b0ffbc1d5e062e86e1341562139"][..]).unwrap()).into_account(), 167 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a3615a414ea68c471c4928a3ef2c6cb5dff899395cf4f4bd9cadc3798b9b2f4a29775d6f829aaa769a992405f69dfad763dbfb2c9328bc5811d98e17d10ffa43"][..]).unwrap()).into_account(), 52329 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5771edb55a1017f329095987b51235a2bc6f49e0f2bc16f4cce621706f1e21b03121a1faa47c7d89a157f3746451891f514304eee4daca3478678aac6b8c9917"][..]).unwrap()).into_account(), 3300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5cbbb4689d4cdac2d4d4929df6dd6e9121284f204e660892694a4cefaa8b18fd37307e449dc220d93d3f31d50bdfe9ca9553395ede5cafb4acf872dfc0ef908a"][..]).unwrap()).into_account(), 257 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["282b831cb00b711e44a8b78183ab394f46e8418fefca462518ce473efe65d01841ba5429256bfb23c26bd2d8267ed69a41fc9063bec43490495986425a6fbef3"][..]).unwrap()).into_account(), 167 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0b41c0bce6ee69109578fd6c19538495ba184b8bf073964f0e9a2af4527cf66585d034445f7cc905abee4e093e178b073c9c522f8278a20b6d8c7ddd931cfbc8"][..]).unwrap()).into_account(), 19967 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e183cf08a0a25536c150efd12eb1bb8857115794b40844df1c54beccdf6130a8fd56fffafc404b63a44491947bc4f002fdc91db914bd8e0bf493306c0d6def08"][..]).unwrap()).into_account(), 9 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["43a7f21db8cbf21fff4831d22669a840df9f5bfb128d743cdf8c032e17d04ede098d447a7555d6b9cfd22e261771c039bd5f115b3d0c069749df4bf539690f3c"][..]).unwrap()).into_account(), 334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["953a0f8ea91d9c6a755b95f55538666135e185161aeaa9c3c9615df2e5fd1389bceafb22cc34996b1667c6c01d76dae3b32ebf9e7baa43534658815533ef9772"][..]).unwrap()).into_account(), 16667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["37617cd2b79f02259fced96ee229240becf6df4f9756773213e6f498d5b7151fd12cd877add38c8e572f6a9d31d60e714fa3647613cb3f48cd8d23c0f2243dc6"][..]).unwrap()).into_account(), 3330 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["51aa8e998d7d99025f41e05f4efe9486dd6c4cf80019d22b9c31b592174adf5852d3ed83660c74742054fda127137fdb3059f58770f5fb6c9cf0ebea9236b61b"][..]).unwrap()).into_account(), 33333 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5965a88f5d6fac0c06628e5e61769dfa71a1ed6042fe3380d67eac5d9239fefbc7f7955225b5c1680d9ddd26b5fec590e8f2aa5e7fa6bf3922b72e0a1fa6056b"][..]).unwrap()).into_account(), 67 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e2751f1a84aa8a3734dfbe5b8fc77d6f92c461d1077486bb927b1d1965058ea0da4959ecbc8ac17b761d57458598230700da3b4f2a8a823042e4cda703f7032e"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["72e3223a0bc0a5d3fad618e349d17eb3aaacebda6ab460aa3300ecadbe5c646db9502fe74797100062931d938929639e0c480b88f607d45e8ce5705615a7553b"][..]).unwrap()).into_account(), 5000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6a81980f36fd213f08a767cc6d06279c16ac4c4164c634abc091937711d43e9bcf10488e6305aad75262035529308e80c3fc8e170b04554185d95b370047624d"][..]).unwrap()).into_account(), 9000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["20e45d2530e4ac1bd9a952e544df7062cb03990716944c2962f62d21de961ecf75a32263d8c0beb8b3ebe480caba1d57f256d7187e4d077790f6a014b7b9220c"][..]).unwrap()).into_account(), 3330 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c9db7e90f4558f6019499f2e65ca91e8e82c9d4bef2ba4eaeb5a465daf0a756552a7ce31613e45b0f187c521fc52ab4e6d764d4c080f155b8537831b868b02f0"][..]).unwrap()).into_account(), 599 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["3d848b4ce58f7dbd97bac04df618dcedba19b397853405469219f069186f8f1af1994ecbd2b6a3bced5f9d2994f09de77b8e2d6fff1c1ad7061e074695acc413"][..]).unwrap()).into_account(), 30 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["62275407d640c379a821dd05da2ce6c6bf5ece026d0cf4561574482a029fff9d7387711a6a5317b23e5f140a32dbe65e18e90b028704957b5671da6830911913"][..]).unwrap()).into_account(), 11 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f2da22f5942290fd9fae5e6b9b2b215753dc8bd87041097fa72d8f584dea11fc5d86a4f979f684ada83d13219b9e8b73a9095137c75e60d72ac4b2c51bd251ea"][..]).unwrap()).into_account(), 334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["cfa9ae0337f827a780e89f1fe69b8de0805aefed2862cba50b675cf069e46c2c96cc6f73d9bb486ad463408b9a519aac02fd3d2dd2d412ff099d15346af2fcc4"][..]).unwrap()).into_account(), 400 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7c89fce093b059a7ce64a7046d60b1f0ab289bb82d8a3b69687464851c8c3e77f0f87254afabcdb1f951fd7050926b543f44ebdba3fe714b807f66d6123a7c69"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1b3f9c9134a8a5a4ca3a73c8aeee6e4b7f5da5c755e306b1752b6964420467f65cb8e2fa43be7fc869731f0a092a5ed7ee079065136718a1a0ce35b48338aace"][..]).unwrap()).into_account(), 16088 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0f06145a534b3b107331928309dcb6200b83a8818fe74aa1f5a91ac1d3ef55c867a8f77e228ca12b8809ed2b0d101eb024ef19eea593bfb2281e254c5f2c94f2"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["63eec5ca2d3f2d6baaabbf7dd64cc306141428f495113cdda42598756facbeadacf9c2a9ab8e74f729a545a0be7390701f0f11b859ec22a26b7aec5127418650"][..]).unwrap()).into_account(), 33667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4a5fabb8b575800ee8e96b7c0575a00c4fd8f82ff2b6f880a10e44c2567bb494deb1c81f0d7b78f03b517b2365fea1f139c42a98e61581dd2f7781bc99639053"][..]).unwrap()).into_account(), 867 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["67ca41019c00d168cd01319a30d69cf4cafd76a2718ca4ddee6faa99fc9855bf8f0ccfbcb042638a342189dd5561e80c8b6a93ca116419f5c91995023e60558d"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["17d08e5cf2716cf40468a31ef5251609277c946dd1141c350202aa2be01ef68b4c7bfe737958e40d2093d5bb5e859346abb85c9af06c2f07c212321e45ce637a"][..]).unwrap()).into_account(), 3334 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["bfcae14d5b4410cdce9a5d71d15010fc6b05ba75162c59d30cc9f349b619dc715e90d0bce231aac8c87bf6a4c6ee740dbfcbce55928cd010ea2d1059dc6765de"][..]).unwrap()).into_account(), 29380 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fd982a3932eaa53bbd76ae7d3d1d7aa29c1f45a25f8aa097795c3654fb4bee1f8ed9b008fe427e2ef92981c06ac9ec88cc098b093296eed57626c3b11b7c64a6"][..]).unwrap()).into_account(), 99400 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["89b32e007b87ec02bb3f1581a53dc465c48d4c3dd7cca3912fbd0fc2b4198f5f254b119d7635e26958189bbc7935e63acb012a442b157a819616dc069039d189"][..]).unwrap()).into_account(), 434 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fc22bcbbdfbf74d4a53222411342d0226f5b8b9bf06b80b3a61f01bece0181b64a8ab245ec79ea97706a026cc8e47550659d94bbbfb11d9c040dcbd8ba0754b3"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d411750dc1dd12005f5de13896bdbc3b59a6ab4de4dcef734fabc1949321e9ebf9e6b4d0a220f47a7586b87dec386d26da2d7198ad9dbe41a49f97b924f14661"][..]).unwrap()).into_account(), 667 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f79c5c10aff30e6a6feb8dec211550a00bec4d2f9746e43c7feb65c6ea060c7830949f70d43f7ac24b9133d6f8da6c3b95e4d670fe89b4853e291488d5b34b85"][..]).unwrap()).into_account(), 43320 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a937a9892a050012e9026147d175412db7adb7de9e42d557b944e317aab2314cab6efe263280c65c09acea5be3d183bfa5a96939ae6ad4666e3f04c569a773ef"][..]).unwrap()).into_account(), 6660 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4ba278a155a985874f1e7e8be6593d20259c15da7a070ea9ed33b41726ca222abef79d34aacb6ee0dbaa8bfd2290d5ac1d4bb028b211896025dd3e3eaa529c31"][..]).unwrap()).into_account(), 788 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f727cb26438acda0f81b81a2e73bacd2958ae33c21a3ac1518c8bd4fbc86e683a3c64861102e5f9bc6bfec17e060b97bb2f66d184e892e77dda3dc207f7e4230"][..]).unwrap()).into_account(), 5000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["27e7e0aade2514f8bffee3cd8b17d18c9c1a785be83b1b2ac6a68e5d9c2895d6d6f5a84f323c625a20372064060693c8376c2ebfae9457305de707daae624bab"][..]).unwrap()).into_account(), 30632 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2dd186240543b760b020217877fe41f1ec55b5fe433fdd68c5045ca1f4c1043a0a86a9a643c46032e7ee56460f810a1ba35e8b4bbee64b9b8d73bdf1cd84e398"][..]).unwrap()).into_account(), 400 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["03665631b5e2e0fdebe0ba175a830583d5c07dfdb1ed06f81455b3cabc086a7817efa73b590a5315bfda1e9543d24a70d96c2c3cc44ed322e7b35930e860ddd7"][..]).unwrap()).into_account(), 13050 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["680ab85ffdb0746a05e9221f39ffb0120d29afa6c175e0966e12c8825339694a3d709bece4afd8d24aafa341ee043b5a7fe0d01049887d812f23928e21cfc5f1"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9393de7d998f4bbd208fd84cf65a422291cdf6cb30105d842fc19134c5b8637de8ac7c31d9d0a26fc46f108a806fdc067fdf2e6de9af947063d314263775bc2f"][..]).unwrap()).into_account(), 5202 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["07b2216eb79681275a76d244e8ca9fee066f45acdb3e1c91f1a0bc3bcdc7590852e07467fa97cbe6e8913f5f40a138ee1011b4257a391bd0b45b242a5155bc19"][..]).unwrap()).into_account(), 2426 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f4bcf3edd598ff8d8aae361964c7c6c2c76d5c289b2038415b3bbe07eb8ad79d9f91de40a8844816b7b6badcb33a65ced695a92f0b2cb0d641b06f539841e3f6"][..]).unwrap()).into_account(), 15698 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6927df714103e2d2a21fa4d1123c4bf688da3da75f4bed7df33000e2ed8bc188429206276de1e4973af6fb6368799fb8d2e54f50e657fb39edcb65f6cf96c870"][..]).unwrap()).into_account(), 14 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1ebbe268bccf1d42d6d5d59481905721913b085d5705f16922bf330069d98c0de18fca94eea55866af928c0ebed1291b60a3fa8541e1e2f12bbe28a10553514b"][..]).unwrap()).into_account(), 37 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e053dbf584e21af9e6ab95ce3c6192b6f74484183c18d4e91ee579145af72e81a5f66938a2359e22e24c1b1039ce8d6ac2c5ce9618169ce007b449550dfae3ff"][..]).unwrap()).into_account(), 375 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["79059c758bdfd19854924e5ed7d008d0763c8c6aa30899be18301394ffaecfb546022d3663cffa0ac9f8f0a58ded7786a186b17e9f3f5b95b9bc90fbe966a6c5"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["21824eafba9179d1c15ac06486cfa21eae55dd16ff952fed8e6e55bd1fcab2e90b28bc3ec7788fa64f507177110b37508c116c240f9c0037075522185914b616"][..]).unwrap()).into_account(), 6000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4d9fb6274ca1472f02fb51ff4c80efac46fd9c010f098f9e83ca71616123910cc7eda992525f588e975d5b0c8860832193adb8b4d47f31cc18b969ef52ba4b00"][..]).unwrap()).into_account(), 7650 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["036d3e8af5484e48f27db6b49abc8f1ca984b777154684ef1b3899a740e10eb2b296fcf101d7bd05461ffcaec3d578b56cb762b3e9039cf577c2a3586774967b"][..]).unwrap()).into_account(), 180 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7d0bb99161371760d6432c294d5ee6241db2c43879a356fb4c730b19955213bf48805d73477e210d288b5e706798361f13c82910038fd1f42b86344e8a080369"][..]).unwrap()).into_account(), 1050 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c0ab19e844c7c981d5170688236e3392817a41471dc41bc55634e09626f76b852328f5caef4d0926204cd56a2eb68d55d188a1582879c588b78a6572541768f2"][..]).unwrap()).into_account(), 112 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fcb3c44cc1e1da6fb246d9abf5febf21457fa70eff85b35cba56ed9c4f71db3ca48387344472803dbf84f296f255cf2cfa454bb334824e0b3448c8d8490b2454"][..]).unwrap()).into_account(), 51 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["53fc06c8c909a3ef9931e0c560e2d2bacfa9b832d271dd05c71cba62c8d93fd45b86a27943e2522dd414bdfa527057c5839ff6483e4763b0c28c929be84194ba"][..]).unwrap()).into_account(), 375 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6f6c0b0d402ac9feec53224b2063cf9c326e22cbebb295bb25eee0c70afab998fd1638b2fc76471de913e6390bb86c6b91cccae081b031e9667d3de29d4d4a6a"][..]).unwrap()).into_account(), 74 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["05692ca992d1e973f65c6495a0115acf3b6ae04ab24f1ac0eb049bb4734b438617a207cb5858a3517b84a49efa5b344a1af2d2e61be5673f24ef86209ababa48"][..]).unwrap()).into_account(), 75 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c5f0a4b4a94b8e007e352b583c293e8582d582061243fbd135c323662962c6206e67ace7530008c2c934b8fc4c66408286bffeae6b6ce49f0f38486efaa9ea08"][..]).unwrap()).into_account(), 450 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["74c4d22b2d5a076bfd0a55cc6e382b1261a30a3726b258addda15386842157a9cf031ca08c501bec22b72d804f2b6b71c0c69358cc4268363f41f9ecd1d7cb19"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["720fc34448ef912e6233292a21275698ca802af88dc2df732c3de727dffa68d3ed166ab30ef8788d8bb94186f99cc413d7b22efb5c63e643ae778e981f47e7be"][..]).unwrap()).into_account(), 763 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["12a15fbf755dd51528470879424810426f645081488b640f2b3e3e9ea3105b529ea1808be20a5cb9df55e13df0da9fe9b01c6937ccbc22299602596bb140d72a"][..]).unwrap()).into_account(), 195 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["626ad8329c4ac28001a11942e69afcc7f036f86ff0465f4a7a19e102e414287d662d3ff5abeb7965a8547f2b643287bd1b11bf0b01aa6dd2280eb7f93f10ef78"][..]).unwrap()).into_account(), 45 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b0d003238dc86ffd2dc2d158aaf035db0aa96cc363f2a0dc976786cebfc91d078ff2098474dcd463da3511f15b91aebb68d622429aa053ae924985b022abedd9"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2f9cba943d25629db78fc4930c0e4e7b5dc4298e902d1936f74676ea4f75a16665831bb3a17d32bec8548b63eb1874253479c015eecad492a8cb15bf8b03def4"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fea355eaa54ade1116fe06abc377255a996111f711b8e2e05a24d679611eef0f4c9d2a98cbba9dfed531452fe4627a2f1b36c7df1ac7c3c3bf3c66d85c9d8dd4"][..]).unwrap()).into_account(), 199 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f83a4e16aefd6ea541d0693d0e69b64ec2c702998a36c3900f545365d86d73d46e3e03b9f7881e8a144cc5ed50839ee44e53ef61639abc1d94cfe4a289deac56"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8de92e232022e468832496fad5e2eaa5beda21af0cd75290c762de17131c3e0b59d222d9ca2a720897221f5b91e11ff315dd3d6412c221ca9b5cdc02846e075b"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0f61974e0a01d8913866f9a55a16999b8e0d618f519d6ee6e38c6eba8b6e63bcec91bcaf6da202cc2278e3f0a9b9df98d5d5915e3c5327b726a1e299e7820437"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["08d905411a688cc8ceea965899d7cf41befad60e4190b1c3f37bbbf4d5f9fe77d6e059242c094925c0ea62001b82660c89fbc84e370545cd481930a8c6f3a942"][..]).unwrap()).into_account(), 650 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d6068b0a7b3304f326afb3999a96d78d363c40abaeb9929ac141ee2eb43947154ae67396e7161e4cde1b56c89cec6269ed8da40c7f0e8d9a377f5a221006a6d1"][..]).unwrap()).into_account(), 225 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["44101c00ee9ece5ead0feffa0283508143e398a46965910f9bd9e38cd07aeb47296fc57bf8a9a2cd2c578f2a32b11ac4a5911f3b18e50408161facc5a70e095f"][..]).unwrap()).into_account(), 675 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["520c8dc249974d92c0e14e932c608952f7b5915085a55acac077a9c0be774b073e231eaf4b900d4e06c7a8c866595b2dc9a9d1557b1f3f9f9e386974e644fab0"][..]).unwrap()).into_account(), 448 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["27ce684ee594ed5e59e22b4c7deb21daa4d56d2bddc9ad0c23fc827d4bad2d66bcda2a903ae954c42e6c13c39696f7f71a6479ca1a003ab37b779b2d96e69d16"][..]).unwrap()).into_account(), 129447 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["cc44d0a1cce09f218336cce30e93da43f18c581f8aeb41bb9c9bec758c8ebbb0ff7f525a2ddb37d003ca2faca772ff8cc44f389d53e14aec5724ecbac74b8dd0"][..]).unwrap()).into_account(), 460 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a26f4090cbed4f348cd31dbc309fe9e8ff05f6600c5ef85d5934559eea8d87802f5184d5cef4063587436a4d2af042ae5dbe6b0a1ecbd7c5c019aba4e83f1b6b"][..]).unwrap()).into_account(), 24 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["79f194d4a261bce82f51bcaee5796d9791ba88ccbd29c59d6c7e49d7d91fb3b7096d412199e2f727f140536d61e97831cddd9862d13b803bd44e9391cf1829ae"][..]).unwrap()).into_account(), 135 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4e82ea48651c9648d694dd9eac47152f27e2c3a46b4c8343862ff8fa7927994c5c7f900cf8b509c09c1cbb19494fdb461131a34a56120e985f072bb3e6004c92"][..]).unwrap()).into_account(), 2250 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["05efb174820c1e9e4c5b9a237785027be21a2ad4578ef08028f9da032fea9acd64ea4239f85bfcc3dbcc0e7105d22aadd43c3ffada90316b9771b8c3e1ac60f8"][..]).unwrap()).into_account(), 3000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["027c7bd9ef503f22e752db1b684ff18ea3cd4179ea6b46dcedb66372e9d4b9aa03be088f43e57f866d16a4cf6d12d02a8d6f24c863f242939eb0e301b54c7664"][..]).unwrap()).into_account(), 11 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a4d3e58338503d3f4982e7b55536da3c6d274121f32453e214f45dc9ba297f06da1f8dc21651d7a4bc6c616f2ee3e7bcd0de202dcb6401f819b16bf48077531b"][..]).unwrap()).into_account(), 75 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["bc59f32126fe34783d108c729d4aa86801306d67d5a7d8d34c737a225277ba0da53e6f1756fef42b194e21aab5cc733d4467c41caa068c04a65c012ba01e294d"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2c6848316051ef7487987bd3eefdc06821df02539649f10bb054daad13697f2479f8aa830d2df1ce32d3f5b2db65532189ff056e932f031deb403106d6ddf1e0"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ecb8527b77a361a1778ae33bc63da2bb8961d9f81410db22ddd7e67d9c65f0a7144b67ef804f2765c6e645ff46d8f64841b7ce8e250923df6fcd6975c53e7ac1"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a44fb914681b4484b959e78848216f0160c9732e999c242f80684cdf09799ee3b3f085532b3d5a0fd96ad34753f690ee90df076e2569036e988aaa754ba1be53"][..]).unwrap()).into_account(), 2700 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b19efb61d64629fdeacbdcf8237865161d8e54e32b60eab0580c26a82273b0509c3847903ed65136064aca5fa10e0e130c5c58bea02487f1152060c6927fd194"][..]).unwrap()).into_account(), 2700 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7f407c086ffe0b9f93cac422e57e8e65507219870aface603275bb4e8b649d936360325c43886036204f869e44e63c5a4844479aaf097b7a54808ec90e520fe1"][..]).unwrap()).into_account(), 4500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fc3d0d5e9f3727aab1eb2989fc74953d27a7e45a20dda1c996504e0db7be8e76712a1d934540cc05e36718a0bc611b4d961797445548b39462a85de429a5f366"][..]).unwrap()).into_account(), 43510 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["26bb69c25bb7131eca2cb3d62031e0c339f2000f4038c4f635be3698a9901de5bcdaec5a9bc8476aca8d63e66c2a23bb54d3e979b51449cf289c29373b097474"][..]).unwrap()).into_account(), 500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["cd7fee8275ba4c8dcae7c4dc1bef79923ed8fd8d529b039ebc4cd89dc39e5f8c4a0e337690f9c7d10c9dcd3d9740f046ec7a83875ce3b2c87a0efd9a882b33fe"][..]).unwrap()).into_account(), 4632 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9e83c058ab2033d094f71d49b7c70ec75a88efa6d4d9ca0850850bad8b5c6f53518492d4b4130988a612ac59dac01906f8d077de7a4289ae0ffce070821e32bd"][..]).unwrap()).into_account(), 675 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fe7dcb408f69241e2fadd9d5678fc70c746cb3452aee0ffacd89dd52bcf44924ed532196c1e0cc4d47339c2da61d400ac748c2a3faaf67babffd5c4f44bc7df3"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["af30d6736246a316794280079ca3eebac801e254364475ed7aec4d5a20f94e37fb9447848919bc5671464f6090e593b7d3a024582e954018d0831cc289a3f458"][..]).unwrap()).into_account(), 900 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5bc3a006764a0650af064c89add4cc9b05a6729dc4f147a8ca25e99b84f706fccfcf4dde37dec244cb33cdcbfcf4ce017d3bb4e88f5a43007f06f71715a321af"][..]).unwrap()).into_account(), 310 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e01c5f66b94fdb173b71ac1e65dc5dda63dd7d7708622cab2609ba80c848356a3f1e7000db776b3b0b9694e014989dd0faad4af019f4bad8d729d408ab34e7c0"][..]).unwrap()).into_account(), 4500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["01975118d6c6ec286fbe7381a205d86d22c28ff370da017158947ce028ab478ee5a09aee2e0bd914699fa22905528d9e34c4d12dd7043061d5c2a34c6f75a507"][..]).unwrap()).into_account(), 1500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["dcf2a553e6cbfce1305f515b14786cb8df886959a7b217054ba1a6339f460186b5a44a19e52312a8b6e3f332d4a68c9d6f95fdace6552f0aafc8d266d800efff"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["3f493d91273d670753b6cf6aa1f124ff7d11a77bb8169f1d8828c4fb6d2c82ab837e135e133f73af1eb8926d572e529d5a822f95327a9aae309aab63349d5a99"][..]).unwrap()).into_account(), 300 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["58cf634a257d5d92133425ff1657cb343777604bff4432c193a0b0ef81eb6d4039b5fb9aa616d873cf0c4814af888e40633880f279b366fd1824b838784c6d24"][..]).unwrap()).into_account(), 31207 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c05636d1a34307a6b327e3c694309d939e2dab333deebd50442ea4d1e40f9e6fab7572ab86b7a067029b5b2cad7b582daefedc62a674b22d3201036a3ea2078b"][..]).unwrap()).into_account(), 12000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["502ba6363ddb0bed765ad87415af6fa7e9bab856789ce8e3618c6454a9ec57222580f7a8392f0959e067d822ea31a60ffe77fb8c361191d8e6487eac346e9857"][..]).unwrap()).into_account(), 1499 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f60fc5d6e78779ffda68af1556658125e7b526152e6cb25e24fdfc4bfca0f107f8076c05d69b0b48e96762b29ce122f8c38a0eefc17665a2744a34f0aab70d84"][..]).unwrap()).into_account(), 325 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b512d80c4c7f10e9cc012ee553787a263257a1560540fc3d48d78ab78015c8b40074f67d8416c4edaf02bd702fba3edbfb95306e4b9ae3926db876fdf2cbbc87"][..]).unwrap()).into_account(), 269 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5986a70b6c8746839ab9b44845d37e9660a9540786614156f12592190b3a79672cbe7b365157628639fbc027d3ebc2c3ad497c71f0466dfe2f366227b44592b4"][..]).unwrap()).into_account(), 130 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8307d1cbb0a30b62fff1ae5afe8c4fa76b1cdff2865d7acfbf6b7a06aa3f376bf54d18db08d0c6f47b2d7fb6a5cf287d6d145163c7db9b874002f3602faf13c0"][..]).unwrap()).into_account(), 7500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["362350dbd18f334bfa9aad028b28d310f858caf3cc6f57a3954de19fa4da0ea8300b427ef34fa31406342146aaba16b22a5fb9bfc85a7c61b93bb927b7ac639a"][..]).unwrap()).into_account(), 298 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["3724f519ef1dc0599adc7a894ed07f375f1db038c0d1f79ef5beb9b50b7102eff8f76511aba738c9ac6ffea625f65ad35b12a9536b125eb6d6e9374ae508a8e1"][..]).unwrap()).into_account(), 1999 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a8539421b6e107d04e09e6ee39c790e30c6d251d0c69b305243330458e5a4ad1397b2a7f2072d1be77cc68ef68f8a6cf1bb1f70e96fd01054b0e8f02b0533222"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["bb72618d35356fd7f67c5a6df1bbd1c4a7b7cd5e7f373622eb8c6fe3b38e9050f68c8a2dbb5f2a1b27b1215715e0bf6b11cd6f48e38275b49c6a965564e7492e"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["cd8a32eb1971b833884869a18d64df830c2eb656e9ec5c34563d7d1a44f45834b3a525cb099ca4616b781b2edd38e1ed8e5b27fc0b501d01863b031435f9b5b8"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["85060584a009b92ab71be70bf43dff7bc72a8ed5efeea479a013f9b3306862d403e68253ceb01b53cfd9cea68e6baa77319c91d49b875ec3113cad4e3dc7de97"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7ce1c1f8923798cf1ee5def144f8de856b7f644cb84769e2a6ec6cf77c0a90a504553c2102a4aaf227b454cd53b1d72af9e1a243b3ac5d99936d62828ec2ca7b"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["797ca98e153b3026a8979cd6cb2dd341e4b16f787338fda7b2239444bebcc1ad78b5290ee92adb49d7e59a5f828f2334bdc5d3a0322a5ce91ea09d584b2c8337"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["de005b588ff2e41b1efcb4c5359dafda54c70811a7d089a2a320b41cdf789cf39665cca63056afe62c40fa7280312167468a9d527dec80d0df974bc0529032ea"][..]).unwrap()).into_account(), 100 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["029d9062260a189ee6810408dbc602f0c2527d1054912170b2878a00e39fb0202aee99fadf3327b4ca9749e0498f4d220661033faedc6873c8916b0b9d3d22da"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fa43b3065aed5d1a762a56c2eb4503d3ddf26867eb7c11272d8a3a36a1ca86c6cafd48172cc1febd18a603c5de73ee849f008e3baf46e52aaa8758f8ef8523fa"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5282a66bbb3398f3735f0ce393102f4132cde6c2eca051a2e97409b7f1e5c73d58e5e39128864c2f5931f9bbf111e6b090998df7dde9bb127cd2574c2f29b6c1"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7b8721737122826772820d8740ecee884da1b3d21f876ff0d3781d8457f4609c61cd3c95737653c33e8848b80f7280c9ac750a011b5a6f2fee66476bbee8c16f"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0088c629b01faba27aa47c33cce6750aba49714058dcbeefa2e9465da49601fc1d38afd637093fada74d1a3d118b5fef3f5e210a99cf06a8290569bedc6ed68a"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6c489eb03aac097146c823f3d2f4e2a0ef4afe9712b1c07a48a7578369428ff9db0654926264cbfe62a4e9dbf33f0380b161d945638a8e59103b01d1b6c734f3"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8b4296072f1dc76ba56f3566eb910c89c6d7bc21bdc46b515fe73abd2cb333c62d300d5db1f120e7748850d71c3624737e44a636afee3dbc775158bfa1018a43"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e1cb1c9cbe022e885c2d32d230e799b1ef4429aa42eb9080cbabf6eb0b67d6dea9d050afac52fd5aa56270278ba613f2fb07f9002cce330b068eb6068d5c30bc"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4d6d8c6367a08d03d3ad8f93f6757e619e333b2f9c97ba4ef00054e27a9974753954f5e58680e1e8806a820f61fa86f49cc0fc6cef0de515b8973ba185b5bea7"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["22e2f3b957f2a97e3377a7234e6fee15a86cac6a7d35115ad6cd8346e5396dfbdbcf09f1b86780b2d606c04fdfb4b748e40b472dc7a67334689b26d0ae1b4ac7"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["215b035ccc0e32254e729b049789c4609466fdf9d272ad404a7a26cbb671cba2e31cd9a132339edfa66805f69a23c0e2179143ef0f96952234a5b285739ec0f2"][..]).unwrap()).into_account(), 1801 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["51a1fb68bedbbb0d71d1b2f37dfef98c68cc424e634d95a390609756375b829a4d3c77b7066bb0cc2bc836b770bbbb88a1601b0e779363c9b3f993ee930265fe"][..]).unwrap()).into_account(), 127 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["891ce67326cf8ca5acc4336c092e0443fa5593bdf4513df5b5282d8fed511982b0db17e277695947f91eb2aee00dc8d835d2921e8ce5994610c411d7267ae02b"][..]).unwrap()).into_account(), 146400 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1af8d9eed1f923b1599df2bd50e2ec2b5a479f68ac64621c6c04ed881776eeac0f681e36ca6ffc5b53eee7951d2c908446ae8cea218f62256caaca1c9561374d"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d666a8fdba8dc58b79b75565e91888f5bc33f879648021a00631c17884073478749224534474a33e7f1eabb6dedf7a4f16abcc67c77030f8d3fbb276963503ae"][..]).unwrap()).into_account(), 14 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["973639da3d86d4b12a1186b0c5d82554a380fc1d44d55f3df1bffeb94873df1464fdd98df15652e6fe67ac341d555dfeab816c40e99f9081ccf92fa8b66ad7a0"][..]).unwrap()).into_account(), 148 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8cd8f20f8862f3b68a9d6988a11799a08d9cf02102850b89c9863db74d2554d24d0236475ec00888b6554b05f78287d0982e2e269076c7ef97c34a0b7da04782"][..]).unwrap()).into_account(), 5000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["21c769aab0cee8184f4ddb5700343d2e6c79f65b31fc31a56c3a313a88f4ce3a4b42394c91a8903a50cfd9d3f25c03f5da969c3b0a0ec0d728b21300cc835d0d"][..]).unwrap()).into_account(), 485 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["81919214c9febb3a9d1cba6f1bff19107f447290b2cce7a8f113efc561f7c8a38ee04b26eeb3498f58d13fc5864106acde4a79109f91adcad5f994f97da06e42"][..]).unwrap()).into_account(), 580 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0cc4ed21767b2a3ddfe6fdad5bccd51d9a4aa0d330e1f556f108176cfba1184a1dc408b06a03053d17a8ccf3f669bcbf1e5bc2e20e087e9780a4072a0725dbb5"][..]).unwrap()).into_account(), 440 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7976aaaa31f617ac69edafe74931128117d5d6705e902e70c15b53c413b91d756dadad375bb26f7429a2c1e71b4f645f6ddd37a4fd1dda8d6cbe3fbcbf8b5e2f"][..]).unwrap()).into_account(), 50 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fe7adcad2dbbf1f5f38aaa1645a82355a1c9d6dfbf1fa3011db50f23d21a4a33f1d63cd7e161c088dedd81d534f9416bfc2b3b4d6d66bf9383adf92d36dec27d"][..]).unwrap()).into_account(), 25 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a3e249729fef91d281ac15f45fd662f7ef2230823de49472e905fdae44f3742053d3820585a579d3ca295d9c01f6e05333bd17c0fdea55398c53e05122bfe64c"][..]).unwrap()).into_account(), 41250 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["ecd8a18ca53cc0ec152a345e870890cf9e1d72b59621e64c5f1464f757d0b5a353c1ab770375e5185b022adfca591e9485dd704db530a2b156bd59f251172623"][..]).unwrap()).into_account(), 100 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d2ed76342aba6a4910df152342f33f1b60bb28f98b2cc473ed57bbf994476bf01f023fcc5a9bad3d68d7ef834fb1260667975af4a0dfef21eb8ea6acd217daae"][..]).unwrap()).into_account(), 30000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["89a491f713afc81ada9e7b3c3d44d073b576494dcfef288e030be9b80d8c890e76f0b0e9619e301812e16a601f47aa87c9e1492a277b86de1852f853111baceb"][..]).unwrap()).into_account(), 100 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6a2713266d66f8410b0334754a5cb05b188cb76e211d7e0c2b31bb67e0597075b165d71ece4c5bfa15258a5070d90ad7a065ffeb8875585b63aecb60e0c61ae2"][..]).unwrap()).into_account(), 60000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["f83c8e8c3ef4a3fac793ad93d5ad71a5fa1b6bb5945e4d641685d6d58cf4fc3080c462c4fd91cf9252554b4abf8f11f63293e3d2e60b8e17324512f00d682468"][..]).unwrap()).into_account(), 100 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4ce4fb56b970d045e19230498a458dadf9736af94ab5cca715484b1fe4c8edc15e56b5e40316820f630e767adf1a656c46afcf123c4e4805c2c88e0e8df66615"][..]).unwrap()).into_account(), 41 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["145ae3776b8462eb289e354581401580b34f3f93518722885baf946766b9f31cd974adcb86091f43ea3307eac27681620595819767edc833a6e02376615df20b"][..]).unwrap()).into_account(), 1001 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["dd6d07c7ca77c3fdacb88a41f1557e4e2b382cec2282c6e2921b549b80d53ec69c6e0289cecc6d665ebe747260f09090099283ba5cca7cd719198eec71894442"][..]).unwrap()).into_account(), 282 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["18704b1d1a43af1811bdd5c990bea0e0bdc2eb316fd65ed5f11896d3d20c77db5bf289264f9d71bdeaa995483d107a6dd61e8d07f5d8dff8b4a1879857f805ec"][..]).unwrap()).into_account(), 3001 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["d5921387da8d3380bd0867eac8c8380c58735e9275dd673f13cb3f8267af13d6bc49f758afbe1356d9adc536d4f92b5ef156e5c531428bd10910d295cc778ede"][..]).unwrap()).into_account(), 1000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["af636de12bb6ce76038e89b6734da6545e55da8f127ba637449f6d872b7b64790e8354927b000d0d76231304d82b7f82d0a8e1695b376b411aeeade71dfebe80"][..]).unwrap()).into_account(), 2549 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["be7992ce5d40dee66f5d2efd0e25020a7f612a94805c6e6075521a393b27ba6bcbcf98396b9cbcafbb13d78f834c7d5563e5eac7cbf2d3a712e1946a3ee28f15"][..]).unwrap()).into_account(), 12998 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["b34db548cb75baecbabd7f7cd7b1f4641f1e5756ca5b1f09cc9f14f438121d5670a991e40e4c0aa25f386b6d8cccfc90671a14b4604f6a56f448e12bf285489e"][..]).unwrap()).into_account(), 3200 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["9c80f1ab28128630682cc4304008a79774d4139b1cb481d37cc7a4fc15c1f69968c04a37af93db0162ebe76372fb8d60e2e70a4ec4ec302080d67688b3f43ce2"][..]).unwrap()).into_account(), 14978 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6ea7ce4e190ef7a8848db2f3ad52bd9509349cfbf4596ba7e8bb355be9b3770695bcbdfb7606b5b710c14ecc40773f676d83cb4722fc1066c8b544d2192c885e"][..]).unwrap()).into_account(), 11002 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["a9894ec1eab96d46216c7b56deb80e3fa6e6a3d5233d2fb5455aa063be8840210d55c8ffd9bc01b63c112425575bb0fb0d85001157a665116fd0e7aff577465c"][..]).unwrap()).into_account(), 196793 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5f272240adf3a93bfcc7ccb2a28a1a4ebc146f7d31835590fda7b066bdf8452e8fa1fdfc264548c95d05eaaf7c39a6449b76573a0f0cd478d43fd10d28a7f8bf"][..]).unwrap()).into_account(), 249 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["66f1ce502c9225fcba7bde31e81754d5fbb8cfe49f9db579be520bae765f57873c86adda013761713a7ff43591cabf5a364fe4d1d102e13d006b7b08ec49e49e"][..]).unwrap()).into_account(), 2001 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["38555ae7cbf6d048257c81dbaec2ca095677b8cb69b385b0644da41600e24a2b52825359ffc20b917c26eb2af6fbca03ba2c1ad54d5a55f79405169a07537e4c"][..]).unwrap()).into_account(), 1000 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["1046c1a4edc7517eeecf284a56e0be13199f284c6ff421da951e2431aaa3e66b870a6e533f06c2c6ab0347711016299c575460e0ac23b90d49abf28ee519a750"][..]).unwrap()).into_account(), 150 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["7af8f084f9bed5ae06e3f51a32fca257324c8a1f80abaf2992bd788d63bf302d346d1a3596999bcece8bd6ecc48bdf572e87a279e07666725068bf4b22ed526a"][..]).unwrap()).into_account(), 13050 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["daecb9a26f171b20317aeff7c44e0f52235f7cbbcdea591754c890796b981eac24317ca43c6d80e3b43665246cdf3ea155b59a983b782baa631b7e1b9df3418a"][..]).unwrap()).into_account(), 15 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e28957f67b2ceb54520bdec058db7aa82650030cef80c692bfd5b94de3e454faa9968aec0e3a375bea8847d4db5660e0344343427bc4670faadc81fcd9155ca8"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["113b7251ab7aba4dc499657dc549500f68ae0bb85d1c3ac5a1d45ff81b2389ff469288273240be93f13306adcf886dadc42bfd99bbe7548c02829b49685305fd"][..]).unwrap()).into_account(), 2681 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0b194207573dd6d483132a637d48bc103336794e1a052ea66656fc26bf12639ca3540f8f847f9d2e7cd77c12fe349839fc6d869cecba28e5f040793497383874"][..]).unwrap()).into_account(), 2 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["018a58b9a493ca642054285ff4670db20d4d7eb21f51c1da3e69f3f03366e9efb05d086736ed2a55f0c7ef0030ee0dc87e0b4938c690b0eba7cef13209425bf9"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["0645a6de403da3ed86bafc04a409a7c4daf61fea6eb55518b0458862e8c8c5fb45a0b189f3e12585a481360659f20f81940ac9724698ee36d49c55374478e543"][..]).unwrap()).into_account(), 5 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["fcfe849a46779c349cb56d2a5949b1bf001a4311fa0e78df00f2ad98b0468382c36d74f261365a526babb07e11d809d1b7e513d0fca6d8553cf23005823a8b70"][..]).unwrap()).into_account(), 2 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6584686f4b33cbaab912435442f73ac171734d7f66568dafa848e6e496231f402b3fd367118fe6822e961c4e8589b2dee7d165dfdc4014c4d9c68217049e0bf2"][..]).unwrap()).into_account(), 530 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["87929c2df46a739209c6eff4dd190b902492c64cd55d69faa229884511facac3435373ebe5f772f4e6822a0dd624bf6f504f3320f0345c93594436fd1d707394"][..]).unwrap()).into_account(), 5 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5b78fd109efcc58aaa7e48a4e9a07ac6a615b23a15d5560b2ec93642a474de4317fbf1d0bac0ffa784f60bd23576442ac1c87f8d0789def77decb6e9f59c798d"][..]).unwrap()).into_account(), 736 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["33a4d44d51d2ce8572fae15be57c915e8835c460b2b49ded425219c2744803f871e161cdf4ff0f184ac1343dbdcb5433788c96315595a5ea7583c9057488c224"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e3c85d8dded2a494b3df00b1a64dc1521667568d666cc35630b4f4cdfb922346c59f7bce0f77ae239ac683bcbb451584a827b99bc5816514f013f548fc9392dc"][..]).unwrap()).into_account(), 6 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["59bd91588c7b5e66d115ce5d2a4fd48b9e6399e4df074e2add9de46a438bdb9ebd40250f549efd986ae14b53bdc8f0ef3fad1dee3933c684d3959219e8656091"][..]).unwrap()).into_account(), 5 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8936623b30754126646f690ac854375a750b10f5c116b39cd6bbf2d0f744787b0c9b806c04660ce06ea87d6db33ca289f2bb88314da784e2ad8bf638a71fba54"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["6d229d9515fb7adc4764a67dff68be8314d99ce5bdbf6b8123d1dae5e6e6d409e6a16d1d202de8807502e2fbcc68299547dfc93fd92edcb0703cdc28e464f863"][..]).unwrap()).into_account(), 9 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["4e3edcda30afd7087d4b4b19140eb23a09f66731dba992bb8481c34375c0927be2bd3a57c9c7199dfe110dd1f422da44bcd9f160e27dc67258bb06c8b1cd894c"][..]).unwrap()).into_account(), 10879 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["bce33ba37f5121471acb0a218e0049b0ca3e57931823754c2167d6dc9e9734cd6052eed1bb2289f8c4b1511c0ccae9d6e297c87ee074e7e39d9ca24683845c34"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["2b9163c4ccaab7de46a5a83980dbebde7a1eec6f161ca959e3e69dd56159b1180b21dbee265c72ae5d1b2e4fecdcc296f164a3b0ea7cc8a7e4775778e56fe477"][..]).unwrap()).into_account(), 1 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["5b33f4a9feced66690fd42fc768b1a3301690bc580711f00c8dfdf10d1570d62ee4cd7a8a3d58dcd9f8e1e9b8f1de5430b1622060f77dcfeee4c70b296674f1e"][..]).unwrap()).into_account(), 1934 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["8c3e3d1d4503f43bf6e624e71bbeb76c871f8c67473f7cd68c62ffbcbb381e35dcdc3809eab706aeee390b67fd9192e7e1f9046f58b9afb00fcaa79f3c6f25fe"][..]).unwrap()).into_account(), 324 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["e2477d19e4fe4e8277e8a36f6dcb2934bf05714d6eb1bc6af5283914187ed97be1209a6c0b88ce941915046ce2c3627748d88b05a4ebd508e7514c8de5373550"][..]).unwrap()).into_account(), 956 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["c8758c068df9893f750680c0d4e683fa73691c1712d6cbb6d7c8664c403550690eb9d42ba87c04cf835eee224260f77cd05242eb56a99be8171d78586274a2b9"][..]).unwrap()).into_account(), 500 * MITO)
          , (MultiSigner::from(Public::from_full(&hex!["da0dc3a8f3151b15c4cf8290b518e052b3091176468671b00da06429443e32c3dfcebf444e65e74a3ca057edac2c0c48ae26de461670746e40a6905abe351e64"][..]).unwrap()).into_account(), 4049 * MITO)
        ];
    }
}

/// Time.
pub mod time {
    use node_primitives::{BlockNumber, Moment};

    /// Since BABE is probabilistic this is the average expected block time that
    /// we are targetting. Blocks will be produced at a minimum duration defined
    /// by `SLOT_DURATION`, but some slots will not be allocated to any
    /// authority and hence no block will be produced. We expect to have this
    /// block time on average following the defined slot duration and the value
    /// of `c` configured for BABE.
    /// This value is only used indirectly to define the unit constants below
    /// that are expressed in blocks. The rest of the code should use
    /// `SLOT_DURATION` instead (like the timestamp module for calculating the
    /// minimum period).
    /// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

    // 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };

    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}
