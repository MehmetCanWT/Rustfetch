use super::*;

pub(super) fn fedora() -> Logo {
    let lines = vec![
        r##"             .',;::::;,'."##,
        r##"         .';:cccccccccccc:;,."##,
        r##"      .;cccccccccccccccccccccc;."##,
        r##"    .:cccccccccccccccccccccccccc:."##,
        r##"  .;ccccccccccccc;$2.:dddl:.$1;ccccccc;."##,
        r##" .:ccccccccccccc;$2OWMKOOXMWd$1;ccccccc:."##,
        r##".:ccccccccccccc;$2KMMc$1;cc;$2xMMc$1;ccccccc:."##,
        r##",cccccccccccccc;$2MMM.$1;cc;$2;WW:$1;cccccccc,"##,
        r##":cccccccccccccc;$2MMM.$1;cccccccccccccccc:"##,
        r##":ccccccc;$2oxOOOo$1;$2MMM000k.$1;cccccccccccc:"##,
        r##"cccccc;$20MMKxdd:$1;$2MMMkddc.$1;cccccccccccc;"##,
        r##"ccccc;$2XMO'$1;cccc;$2MMM.$1;cccccccccccccccc'"##,
        r##"ccccc;$2MMo$1;ccccc;$2MMW.$1;ccccccccccccccc;"##,
        r##"ccccc;$20MNc.$1ccc$2.xMMd$1;ccccccccccccccc;"##,
        r##"cccccc;$2dNMWXXXWM0:$1;cccccccccccccc:,"##,
        r##"cccccccc;$2.:odl:.$1;cccccccccccccc:,."##,
        r##"ccccccccccccccccccccccccccccc:'."##,
        r##":ccccccccccccccccccccccc:;,.."##,
        r##" ':cccccccccccccccc::;,."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: FEDORA_BLUE,
        colors: vec![FEDORA_BLUE, WHITE],
        width,
    }
}

pub(super) fn debian() -> Logo {
    let lines = vec![
        r##"        $2_,met$$$$$$$$$$gg."##,
        r##"     ,g$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$P."##,
        r##"   ,g$$$$P""       """Y$$$$."."##,
        r##"  ,$$$$P'              `$$$$$$."##,
        r##"',$$$$P       ,ggs.     `$$$$b:"##,
        r##"`d$$$$'     ,$P"'   $1.$2    $$$$$$"##,
        r##" $$$$P      d$'     $1,$2    $$$$P"##,
        r##" $$$$:      $$$.   $1-$2    ,d$$$$'"##,
        r##" $$$$;      Y$b._   _,d$P'"##,
        r##" Y$$$$.    $1`.$2`"Y$$$$$$$$P"'"##,
        r##" `$$$$b      $1"-.__"##,
        r##"  $2`Y$$$$b"##,
        r##"   `Y$$$$."##,
        r##"     `$$$$b."##,
        r##"       `Y$$$$b."##,
        r##"         `"Y$$b._"##,
        r##"             `"""""##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: DEBIAN_RED,
        colors: vec![WHITE, DEBIAN_RED],
        width,
    }
}

pub(super) fn ubuntu() -> Logo {
    let lines = vec![
        r##"                             ...."##,
        r##"              $2.',:clooo:  $1.:looooo:."##,
        r##"           $2.;looooooooc  $1.oooooooooo'"##,
        r##"        $2.;looooool:,''.  $1:ooooooooooc"##,
        r##"       $2;looool;.         $1'oooooooooo,"##,
        r##"      $2;clool'             $1.cooooooc.  $2,,"##,
        r##"         $2...                $1......  $2.:oo,"##,
        r##"  $1.;clol:,.                        $2.loooo'"##,
        r##" $1:ooooooooo,                        $2'ooool"##,
        r##"$1'ooooooooooo.                        $2loooo."##,
        r##"$1'ooooooooool                         $2coooo."##,
        r##" $1,loooooooc.                        $2.loooo."##,
        r##"   $1.,;;;'.                          $2;ooooc"##,
        r##"       $2...                         $2,ooool."##,
        r##"    $2.cooooc.              $1..',,'.  $2.cooo."##,
        r##"      $2;ooooo:.           $1;oooooooc.  $2:l."##,
        r##"       $2.coooooc,..      $1coooooooooo."##,
        r##"         $2.:ooooooolc:. $1.ooooooooooo'"##,
        r##"           $2.':loooooo;  $1,oooooooooc"##,
        r##"               $2..';::c'  $1.;loooo:'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: UBUNTU_ORANGE,
        colors: vec![UBUNTU_ORANGE, WHITE],
        width,
    }
}

pub(super) fn mint() -> Logo {
    let lines = vec![
        r##"            $2_.-ppOOOOOOqq-._"##,
        r##"         .oOOOOPPPPPPPPPPOOOOo."##,
        r##"      .oOOOO$1.=oOOOOOOOOOOo=.$2OOOOo."##,
        r##"    .:OOO$1.=oOOOOOOOOOOOOOOOOo=.$2OOO:."##,
        r##"   .OOO$1.OOOOOOOOOOOOOOOOOOOOOOOO.$2OOO."##,
        r##"  .OOO$1.OO    OOO:´   `::´    `:OOO.$2OO:"##,
        r##" .OOO$1.OOO    OO                OOO.$2OOO:"##,
        r##" OOO$1.OOOO    OO    oo    oo    OOOO.$2OOO"##,
        r##":OOO$1:OOOO    OO    OO    OO    OOOO:$2OOO:"##,
        r##":OOO$1:OOOO    OO    OO    OO    OOOO:$2OOO:"##,
        r##"'OOO$1'OOOO    OO    OO    OO    OOOO'$2OOO'"##,
        r##" OOO$1'OOOO    OO____OO____OO    OOOO'$2OOO'"##,
        r##" 'OOO$1'OOO    'OOOOOOOOOOOO'    OOOO'$2OOO"##,
        r##"  'OOO$1'OOO                    .OOO'$2OOO'"##,
        r##"   'OOO$1'OOOO:ooooooooooooooo:OOOO'$2OOO'"##,
        r##"    ':OOOo$1'=OOOOOOOOOOOOOOOOO='$2oOOO:'"##,
        r##"      ':OOOOo$1'=OOOOOOOOOOO='$2oOOOO:'"##,
        r##"         ``-OOOOooooooooooOOOO-´´"##,
        r##"             ```-=:OOOO:=-´´´"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: MINT_GREEN,
        colors: vec![MINT_GREEN, WHITE],
        width,
    }
}

pub(super) fn pop() -> Logo {
    let lines = vec![
        r##"             /////////////"##,
        r##"         /////////////////////"##,
        r##"      ///////$2*767$1////////////////"##,
        r##"    //////$27676767676*$1//////////////"##,
        r##"   /////$276767$1//$27676767$1//////////////"##,
        r##"  /////$2767676$1///$2*76767$1///////////////"##,
        r##" ///////$2767676$1///$276767$1.///$27676*$1///////"##,
        r##"/////////$2767676$1//$276767$1///$2767676$1////////"##,
        r##"//////////$276767676767$1////$276767$1/////////"##,
        r##"///////////$276767676$1//////$27676$1//////////"##,
        r##"////////////,$27676$1,///////$2767$1///////////"##,
        r##"/////////////*$27676$1///////$276$1////////////"##,
        r##"///////////////$27676$1////////////////////"##,
        r##" ///////////////$27676$1///$2767$1////////////"##,
        r##"  //////////////////////$2'$1////////////"##,
        r##"   //////$2.7676767676767676767,$1//////"##,
        r##"    /////$2767676767676767676767$1/////"##,
        r##"      ///////////////////////////"##,
        r##"         /////////////////////"##,
        r##"             /////////////"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: POP_TEAL,
        colors: vec![POP_TEAL, POP_ORANGE],
        width,
    }
}

pub(super) fn opensuse() -> Logo {
    let lines = vec![
        r##"          ,...,"##,
        r##"     .,:lloooooc;."##,
        r##"   ,ool'     oo,;oo:"##,
        r##" .lo'        oo.   oo:"##,
        r##".oo.         oo.    oo:"##,
        r##":ol          oo.    'oo"##,
        r##":oo         .oo.    .oo."##,
        r##".oooooooooooooo.    .oo."##,
        r##" ;oo.               .oo."##,
        r##"  'oo,              .oo."##,
        r##"    "ooc,',,,,,,,,,,:ooc,,,,,,,,,,,"##,
        r##"       ':cooooooooooooooooooooooooool;."##,
        r##"                    .oo.             .oo;"##,
        r##"                    .oo.               .oo."##,
        r##"                    .oo.    'oooooooooo:ooo."##,
        r##"                    .oo.    'oo.         col"##,
        r##"                    .oo'    'oo          col"##,
        r##"                     coo    'oo          oo'"##,
        r##"                      coc   'oo        .lo,"##,
        r##"                       `oo, 'oo      .:oo"##,
        r##"                         'ooooc,, ,:lol"##,
        r##"                            `''"clc"'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: SUSE_GREEN,
        colors: vec![SUSE_GREEN],
        width,
    }
}

pub(super) fn kali() -> Logo {
    let lines = vec![
        r##".............."##,
        r##"            ..,;:ccc,."##,
        r##"          ......''';lxO."##,
        r##".....''''..........,:ld;"##,
        r##"           .';;;:::;,,.x,"##,
        r##"      ..'''.            0Xxoc:,.  ..."##,
        r##"  ....                ,ONkc;,;cokOdc',."##,
        r##" .                   OMo           ':$2dd$1o."##,
        r##"                    dMc               :OO;"##,
        r##"                    0M.                 .:o."##,
        r##"                    ;Wd"##,
        r##"                     ;XO,"##,
        r##"                       ,d0Odlc;,.."##,
        r##"                           ..',;:cdOOd::,."##,
        r##"                                    .:d;.':;."##,
        r##"                                       'd,  .'"##,
        r##"                                         ;l   .."##,
        r##"                                          .o"##,
        r##"                                            c"##,
        r##"                                            .'"##,
        r##"                                             ."##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: KALI_BLUE,
        colors: vec![KALI_BLUE, WHITE],
        width,
    }
}

pub(super) fn redhat() -> Logo {
    let lines = vec![
        r##"           .MMM..:MMMMMMM"##,
        r##"          MMMMMMMMMMMMMMMMMM"##,
        r##"          MMMMMMMMMMMMMMMMMMMM."##,
        r##"         MMMMMMMMMMMMMMMMMMMMMM"##,
        r##"        ,MMMMMMMMMMMMMMMMMMMMMM:"##,
        r##"        MMMMMMMMMMMMMMMMMMMMMMMM"##,
        r##"  .MMMM'  MMMMMMMMMMMMMMMMMMMMMM"##,
        r##" MMMMMM    `MMMMMMMMMMMMMMMMMMMM."##,
        r##"MMMMMMMM      MMMMMMMMMMMMMMMMMM ."##,
        r##"MMMMMMMMM.       `MMMMMMMMMMMMM' MM."##,
        r##"MMMMMMMMMMM.                     MMMM"##,
        r##"`MMMMMMMMMMMMM.                 ,MMMMM."##,
        r##" `MMMMMMMMMMMMMMMMM.          ,MMMMMMMM."##,
        r##"    MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM"##,
        r##"      MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM:"##,
        r##"         MMMMMMMMMMMMMMMMMMMMMMMMMMMMMM"##,
        r##"            `MMMMMMMMMMMMMMMMMMMMMMMM:"##,
        r##"                ``MMMMMMMMMMMMMMMMM'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: REDHAT_RED,
        colors: vec![REDHAT_RED],
        width,
    }
}

pub(super) fn rocky() -> Logo {
    let lines = vec![
        r##"          __wgliliiligw_,"##,
        r##"       _williiiiiiliilililw,"##,
        r##"     _%iiiiiilililiiiiiiiiiii_"##,
        r##"   .Qliiiililiiiiiiililililiilm."##,
        r##"  _iiiiiliiiiiililiiiiiiiiiiliil,"##,
        r##" .lililiiilililiiiilililililiiiii,"##,
        r##"_liiiiiiliiiiiiiliiiiiF{iiiiiilili,"##,
        r##"jliililiiilililiiili@`  ~ililiiiiiL"##,
        r##"iiiliiiiliiiiiiili>`      ~liililii"##,
        r##"liliiiliiilililii`         -9liiiil"##,
        r##"iiiiiliiliiiiii~             "4lili"##,
        r##"4ililiiiiilil~|      -w,       )4lf"##,
        r##"-liiiiililiF'       _liig,       )'"##,
        r##" )iiiliii@`       _QIililig,"##,
        r##"  )iiii>`       .Qliliiiililw"##,
        r##"   )<>~       .mliiiiiliiiiiil,"##,
        r##"            _gllilililiililii~"##,
        r##"           giliiiiiiiiiiiiT`"##,
        r##"          -^~$ililili@~~'"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: SUSE_GREEN,
        colors: vec![SUSE_GREEN, REDHAT_RED],
        width,
    }
}

pub(super) fn almalinux() -> Logo {
    let lines = vec![
        r##"$1         'c:."##,
        r##"$1        lkkkx, ..       $2..   ,cc,"##,
        r##"$1        okkkk:ckkx'  $2.lxkkx.okkkkd"##,
        r##"$1        .:llcokkx'  $2:kkkxkko:xkkd,"##,
        r##"$1      .xkkkkdood:  $2;kx,  .lkxlll;"##,
        r##"$1       xkkx.       $2xk'     xkkkkk:"##,
        r##"$1       'xkx.       $2xd      .....,."##,
        r##"$3      .. $1:xkl'     $2:c      ..''.."##,
        r##"$3    .dkx'  $1.:ldl:'. $2'  $4':lollldkkxo;"##,
        r##"$3  .''lkkko'                     $4ckkkx."##,
        r##"$3'xkkkd:kkd.       ..  $5;'        $4:kkxo."##,
        r##"$3,xkkkd;kk'      ,d;    $5ld.   $4':dkd::cc,"##,
        r##"$3 .,,.;xkko'.';lxo.      $5dx,  $4:kkk'xkkkkc"##,
        r##"$3     'dkkkkkxo:.        $5;kx  $4.kkk:;xkkd."##,
        r##"$3       .....   $5.;dk:.   $5lkk.  $4:;,"##,
        r##"             $5:kkkkkkkdoxkkx"##,
        r##"              ,c,,;;;:xkkd."##,
        r##"                ;kkkkl..."##,
        r##"                ;kkkkl"##,
        r##"                 ,od;"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: DEBIAN_RED,
        colors: vec![DEBIAN_RED, TUX_YELLOW, SUSE_GREEN, ARCH_CYAN, FEDORA_BLUE],
        width,
    }
}

pub(super) fn centos() -> Logo {
    let lines = vec![
        r##"                 .."##,
        r##"               .PLTJ."##,
        r##"              <><><><>"##,
        r##"     $2KKSSV' 4KKK $1LJ$4 KKKL.'VSSKK"##,
        r##"     $2KKV' 4KKKKK $1LJ$4 KKKKAL 'VKK"##,
        r##"     $2V' ' 'VKKKK $1LJ$4 KKKKV' ' 'V"##,
        r##"     $2.4MA.' 'VKK $1LJ$4 KKV' '.4Mb."##,
        r##"   $4. $2KKKKKA.' 'V $1LJ$4 V' '.4KKKKK $3."##,
        r##" $4.4D $2KKKKKKKA.'' $1LJ$4 ''.4KKKKKKK $3FA."##,
        r##"$4<QDD ++++++++++++  $3++++++++++++ GFD>"##,
        r##" '$4VD $3KKKKKKKK'.. $2LJ $1..'KKKKKKKK $3FV"##,
        r##"   $4' $3VKKKKK'. .4 $2LJ $1K. .'KKKKKV $3'"##,
        r##"      $3'VK'. .4KK $2LJ $1KKA. .'KV'"##,
        r##"     $3A. . .4KKKK $2LJ $1KKKKA. . .4"##,
        r##"     $3KKA. 'KKKKK $2LJ $1KKKKK' .4KK"##,
        r##"     $3KKSSA. VKKK $2LJ $1KKKV .4SSKK"##,
        r##"              $2<><><><>"##,
        r##"               $2'MKKM'"##,
        r##"                 $2''"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: REDHAT_RED,
        colors: vec![REDHAT_RED, SUSE_GREEN, FEDORA_BLUE, GENTOO_PURPLE],
        width,
    }
}

pub(super) fn zorin() -> Logo {
    let lines = vec![
        r##"        `osssssssssssssssssssso`"##,
        r##"       .osssssssssssssssssssssso."##,
        r##"      .+oooooooooooooooooooooooo+."##,
        r##""##,
        r##""##,
        r##"  `::::::::::::::::::::::.         .:`"##,
        r##" `+ssssssssssssssssss+:.`     `.:+ssso`"##,
        r##".ossssssssssssssso/.       `-+ossssssso."##,
        r##"ssssssssssssso/-`      `-/osssssssssssss"##,
        r##".ossssssso/-`      .-/ossssssssssssssso."##,
        r##" `+sss+:.      `.:+ssssssssssssssssss+`"##,
        r##"  `:.         .::::::::::::::::::::::`"##,
        r##""##,
        r##""##,
        r##"      .+oooooooooooooooooooooooo+."##,
        r##"       -osssssssssssssssssssssso-"##,
        r##"        `osssssssssssssssssssso`"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: ZORIN_BLUE,
        colors: vec![ZORIN_BLUE],
        width,
    }
}

pub(super) fn raspbian() -> Logo {
    let lines = vec![
        r##"   $2`.::///+:/-.        --///+//-:`"##,
        r##" `+oooooooooooo:   `+oooooooooooo:"##,
        r##"  /oooo++//ooooo:  ooooo+//+ooooo."##,
        r##"  `+ooooooo:-:oo-  +o+::/ooooooo:"##,
        r##"   `:oooooooo+``    `.oooooooo+-"##,
        r##"     `:++ooo/.        :+ooo+/.`$1"##,
        r##"        ...`  `.----.` ``.."##,
        r##"     .::::-``:::::::::.`-:::-`"##,
        r##"    -:::-`   .:::::::-`  `-:::-"##,
        r##"   `::.  `.--.`  `` `.---.``.::`"##,
        r##"       .::::::::`  -::::::::` `"##,
        r##" .::` .:::::::::- `::::::::::``::."##,
        r##"-:::` ::::::::::.  ::::::::::.`:::-"##,
        r##"::::  -::::::::.   `-::::::::  ::::"##,
        r##"-::-   .-:::-.``....``.-::-.   -::-"##,
        r##" .. ``       .::::::::.     `..`.."##,
        r##"   -:::-`   -::::::::::`  .:::::`"##,
        r##"   :::::::` -::::::::::` :::::::."##,
        r##"   .:::::::  -::::::::. ::::::::"##,
        r##"    `-:::::`   ..--.`   ::::::."##,
        r##"      `...`  `...--..`  `...`"##,
        r##"            .::::::::::"##,
        r##"             `.-::::-`"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: RASPBIAN_RED,
        colors: vec![RASPBIAN_GREEN, RASPBIAN_RED],
        width,
    }
}

pub(super) fn fedora_small() -> Logo {
    let lines = vec![
        r##"        ,'''''."##,
        r##"       |   ,.  |"##,
        r##"       |  |  '_'"##,
        r##"  ,....|  |.."##,
        r##".'  ,_;|   ..'"##,
        r##"|  |   |  |"##,
        r##"|  ',_,'  |"##,
        r##" '.     ,'"##,
        r##"   '''''"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: FEDORA_BLUE,
        colors: vec![FEDORA_BLUE],
        width,
    }
}

pub(super) fn ubuntu_small() -> Logo {
    let lines = vec![
        r##"       $2..;,; $1.,;,."##,
        r##"    $2.,lool: $1.ooooo,"##,
        r##"   $2;oo;:    $1.coool."##,
        r##" $1....         $1''' $2,l;"##,
        r##"$1:oooo,            $2'oo."##,
        r##"$1looooc            $2:oo'"##,
        r##" $1'::'             $2,oo:"##,
        r##"   $2,.,       $1.... $2co,"##,
        r##"    $2lo:;.   $1:oooo; $2."##,
        r##"     $2':ooo; $1cooooc"##,
        r##"        $2'''  $1''''"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: UBUNTU_ORANGE,
        colors: vec![WHITE, UBUNTU_ORANGE],
        width,
    }
}

pub(super) fn debian_small() -> Logo {
    let lines = vec![
        r##"  _____"##,
        r##" /  __ \"##,
        r##"|  /    |"##,
        r##"|  \___-"##,
        r##"-_"##,
        r##"  --_"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: DEBIAN_RED,
        colors: vec![DEBIAN_RED],
        width,
    }
}

pub(super) fn nobara() -> Logo {
    let lines = vec![
        r##"⢀⣤⣴⣶⣶⣶⣦⣤⡀⠀⣀⣠⣤⣴⣶⣶⣶⣶⣶⣶⣶⣶⣤⣤⣀⡀"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣤⡀"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣄"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣄"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⠋⠉⠁⠀⠀⠉⠉⠛⠿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⠁⠀⠀⠀⢀⣀⣀⡀⠀⠀⠀⠈⢻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡏⠀⠀⠀⢠⣾⣿⣿⣿⣿⣷⡄⠀⠀⠀⠻⠿⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠁⠀⠀⠀⣿⣿⣿⣿⣿⣿⣿⡇⠀⠀⠀⠀⠀⣀⣀⣬⣽⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀⠈⠻⢿⣿⣿⡿⠟⠁⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣤⣤⣄⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣄⠀⠀⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣇⠀⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿"##,
        r##"⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⠛⠉⠉⠛⠛⢿⣿⣿⠀⠀⠀⠀⠀⠸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿"##,
        r##"⠘⢿⣿⣿⣿⣿⣿⣿⣿⡿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠈⢿⠀⠀⠀⠀⠀⠀⠙⢿⣿⣿⣿⣿⣿⣿⣿⠟⠁"##,
        r##"  ⠈⠙⠛⠛⠛⠋⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠛⠛⠛⠉⠁"##,
    ];
    let width = compute_logo_width(&lines);
    Logo {
        lines,
        color: NOBARA_WHITE,
        colors: vec![NOBARA_WHITE],
        width,
    }
}
