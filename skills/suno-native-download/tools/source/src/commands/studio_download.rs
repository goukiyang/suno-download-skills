//! Studio-only original-song WAV downloads. No Library authorization or generation.
use std::{path::{Path, PathBuf}, time::Duration};
use clap::Parser;
use serde::{Deserialize, Serialize};
use crate::{api::{SunoClient, types::{BillingInfo, Clip}}, auth::AuthState};

#[derive(Parser)]
#[command(name="sunox-studio-download", version, about="下载已有自有Suno原曲为原生WAV；仅使用Premier Studio权益，不走普通下载授权。")]
struct Args {
    /// 已有原曲UUID或https://suno.com/song/UUID
    source: String,
    /// 新的绝对.wav路径；已有文件一律保留
    output: Option<PathBuf>,
    /// 一次接入：明确授权的单一Chrome配置目录；源曲用于核对所属账号
    #[arg(long)]
    connect_chrome_profile: Option<PathBuf>,
    /// 可选：必须与返回的原曲标题完全一致
    #[arg(long)]
    expect_title: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
struct Balance {
    credits: u64,
    total_credits_left: u64,
    download_limit: u64,
    download_used: u64,
    additional_download_remaining: u64,
}
fn balance(info: &BillingInfo) -> Result<Balance, &'static str> {
    let usage=info.download_usage.as_ref().ok_or("账单未给出完整下载额度，未继续下载")?;
    Ok(Balance {credits:info.credits,total_credits_left:info.total_credits_left,
        download_limit:usage.current_period_downloads_limit,download_used:usage.current_period_downloads_used,
        additional_download_remaining:usage.additional_download_remaining})
}
fn identity(client:&SunoClient,clip:&Clip,id:&str,expected:Option<&str>) -> Result<(), &'static str> {
    if clip.id!=id || expected.is_some_and(|title|title!=clip.title) {return Err("歌曲ID或标题与选定来源不符，未准备下载");}
    let actor=client.authenticated_user_id().ok_or("当前认证缺少可核对的账号身份，未准备下载")?;
    let owner=clip.extra.get("user_id").and_then(|value|value.as_str()).ok_or("原曲缺少可核对的所属账号，未准备下载")?;
    if actor!=owner {return Err("当前认证账号不是这首原曲的所属账号，未准备下载");}
    Ok(())
}
fn clip_id(source:&str)->Result<String,&'static str> {
    let value=if source.starts_with("https://") {
        let url=reqwest::Url::parse(source).map_err(|_|"原曲链接格式无效")?;
        if url.origin().ascii_serialization()!="https://suno.com"||url.query().is_some()||url.fragment().is_some()||!url.username().is_empty()||url.password().is_some(){return Err("只接受无附加参数的Suno原曲链接");}
        url.path().strip_prefix("/song/").ok_or("只接受Suno原曲song链接")?.to_owned()
    }else {source.to_owned()};
    uuid::Uuid::parse_str(&value).map(|id|id.to_string()).map_err(|_|"原曲UUID无效")
}
fn output_guard(output:&Path)->Result<(), &'static str> {
    if !output.is_absolute()||output.extension().and_then(|x|x.to_str())!=Some("wav"){return Err("输出须为新的绝对.wav路径");}
    match std::fs::symlink_metadata(output) {Ok(_)=>Err("目标已存在，未读取登录态或覆盖原件"),Err(error)if error.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(_)=>Err("无法安全核对目标文件，未继续")}
}
#[derive(Deserialize)]
struct Prepared { #[serde(default)] ok:bool, status:Option<String>, download_url:Option<String> }
fn valid_file_url(raw:&str)->bool {
    let Ok(url)=reqwest::Url::parse(raw) else{return false};
    #[cfg(test)] if url.scheme()=="http"&&url.host_str()==Some("127.0.0.1"){return true;}
    url.scheme()=="https" && url.username().is_empty() && url.password().is_none()
        && url.host_str().is_some_and(|host|host!="localhost"&&host.parse::<std::net::IpAddr>().is_err())
}
async fn prepare(client:&SunoClient,id:&str,interval:Duration)->Result<String,&'static str> {
    let path=format!("/api/studio/clip/{id}/download?format=wav");
    loop {
        let prepared:Prepared=client.with_auth_retry(||async {
            let response=client.get(&path).send().await?;
            let response=client.check_response(response).await?;
            Ok(response.json().await?)
        }).await.map_err(|_|"Studio准备请求未成功；未改用普通下载或继续重试")?;
        match (prepared.ok,prepared.status.as_deref()) {
            (true,Some("ready"))=>return prepared.download_url.filter(|url|valid_file_url(url)).ok_or("Studio未返回有效文件地址，未继续"),
            (_,Some("processing"|"rate_limited"))=>tokio::time::sleep(interval).await,
            _=>return Err("Studio尚未授权准备原生WAV，未继续"),
        }
    }
}
async fn operation(client:&SunoClient,id:&str,output:&Path,expected:Option<&str>,interval:Duration)->Result<serde_json::Value,&'static str> {
    output_guard(output)?;
    let bill=client.billing_info().await.map_err(|_|"现有登录态无法通过账单核对；未准备下载")?;
    let plan=bill.plan.name.trim().to_ascii_lowercase();
    if !bill.is_active || !matches!(plan.as_str(),"premier"|"premier plan") {return Err("当前账号未明确具备Premier Studio权益，未准备下载");}
    let before=balance(&bill)?;
    let clip=client.get_clip(id).await.map_err(|_|"未能读取选定原曲身份，未准备下载")?.ok_or("选定原曲不存在，未准备下载")?;
    identity(client,&clip,id,expected)?;
    let url=tokio::time::timeout(Duration::from_secs(180),prepare(client,id,interval)).await.map_err(|_|"Studio准备超过180秒，已停止")??;
    // The existing independent downloader sends no Suno authentication headers.
    let parent=output.parent().ok_or("目标目录无效")?;
    std::fs::create_dir_all(parent).map_err(|_|"不能创建目标目录")?;
    let staging=tempfile::Builder::new().prefix(".suno-studio-").tempdir_in(parent).map_err(|_|"不能建立阶段目录")?;
    let staged=crate::media::download_clip_url(&clip,staging.path().to_str().ok_or("阶段目录无效")?,&url,"wav",false,true).await.map_err(|_|"原生WAV文件未完整保存；未覆盖原件")?;
    let after_bill=client.billing_info().await.map_err(|_|"文件已暂存但下载后额度无法核对，未交付")?;
    let after=balance(&after_bill)?;
    identity(client,&clip,id,expected)?;
    if before!=after{return Err("账号额度发生变化，未交付文件；停止后续下载");}
    std::fs::hard_link(&staged,output).map_err(|_|"目标已出现或不能排他保存，未覆盖原件")?;
    Ok(serde_json::json!({"tool_version":env!("CARGO_PKG_VERSION"),"status":"saved_unverified","clip_id":id,"source_title":clip.title,
        "actor_owner_matches":true,"plan":"Premier","before":before,"after":after,"quota_unchanged":true,
        "output":output,"bytes":std::fs::metadata(output).map_err(|_|"保存后文件无法读取")?.len(),
        "entry":"Studio original clip GET WAV","next":"check_audio.py --expected-seconds；本结果不证明听感"}))
}
fn existing_auth_guard(existing:Option<&AuthState>, candidate:&AuthState)->Result<(), &'static str> {
    if let Some(previous)=existing {
        let old=previous.account_user_id().ok_or("已有Sunox认证归属不明；未覆盖原登录态")?;
        let new=candidate.account_user_id().ok_or("新会话缺少账号身份；未保存")?;
        if old!=new {return Err("已有Sunox认证属于不同账号；未覆盖原登录态");}
    }
    Ok(())
}
// Deliberately use direct authenticated GETs, without auth retry or persistence,
// while proving a newly imported candidate. Clerk rejection never triggers scanning.
async fn validate_import(client:&SunoClient,id:&str,expected:Option<&str>)->Result<Balance,&'static str> {
    let response=client.get("/api/billing/info/").send().await.map_err(|_|"新会话账单核对未成功；未保存")?;
    let response=client.check_response(response).await.map_err(|_|"新会话未获账单访问授权；未保存")?;
    let bill:BillingInfo=response.json().await.map_err(|_|"新会话账单内容无法核对；未保存")?;
    if !bill.is_active || !matches!(bill.plan.name.trim().to_ascii_lowercase().as_str(),"premier"|"premier plan") {return Err("新会话未明确具备Premier Studio权益；未保存");}
    let snapshot=balance(&bill)?;
    let response=client.get(&format!("/api/clip/{id}")).send().await.map_err(|_|"原曲身份核对未成功；未保存")?;
    let response=client.check_response(response).await.map_err(|_|"原曲身份核对未获授权；未保存")?;
    let clip:Clip=response.json().await.map_err(|_|"原曲身份内容无法核对；未保存")?;
    identity(client,&clip,id,expected)?;
    Ok(snapshot)
}
async fn connect_explicit_profile(profile:&Path,id:&str,expected:Option<&str>)->Result<serde_json::Value,&'static str> {
    crate::auth::explicit_profile_guard(profile)?;
    let existing=match AuthState::load(){Ok(state)=>Some(state),Err(crate::core::CliError::AuthMissing)=>None,Err(_)=>return Err("已有Sunox认证无法安全读取；未导入或覆盖")};
    if existing.as_ref().is_some_and(|state|state.account_user_id().is_none()){return Err("已有Sunox认证归属不明；未导入或覆盖");}
    let browser=crate::auth::extract_explicit_chrome_profile(profile)?;
    let http=crate::net::http::clerk_client().map_err(|_|"不能建立Sunox正常认证连接；未保存")?;
    let (session_id,jwt)=crate::auth::clerk_token_exchange(&http,&browser.clerk_client_cookie,browser.browser_environment.as_ref()).await.map_err(|_|"指定配置会话缺失、无效或正常刷新被拒绝；未保存，未启动浏览器")?;
    let candidate=AuthState {jwt:Some(jwt),cookie:Some(browser.cookie_header),session_id:Some(session_id),device_id:browser.device_id.or_else(||Some(uuid::Uuid::new_v4().to_string())),browser_environment:browser.browser_environment,clerk_client_cookie:Some(browser.clerk_client_cookie)};
    existing_auth_guard(existing.as_ref(),&candidate)?;
    let client=SunoClient::new_for_auth_validation(candidate).map_err(|_|"不能建立Sunox认证核对连接；未保存")?;
    let snapshot=validate_import(&client,id,expected).await?;
    let verified=client.auth_state_snapshot();
    existing_auth_guard(existing.as_ref(),&verified)?;
    verified.save_if_unchanged(existing.as_ref()).map_err(|_|"原Sunox认证位置不能安全保存或已变化；未覆盖")?;
    Ok(serde_json::json!({"tool_version":env!("CARGO_PKG_VERSION"),"status":"connected","profile":"explicit authorized Chrome profile","actor_owner_matches":true,"reference_clip_id":id,"plan":"Premier","balance":snapshot,"download_requested":false}))
}
pub async fn standalone_main()->i32 {
    let args=match Args::try_parse(){Ok(args)=>args,Err(error)=>{let code=if error.use_stderr(){2}else{0};let _=error.print();return code;}};
    let result=async {
        let id=clip_id(&args.source)?;
        if let Some(profile)=args.connect_chrome_profile.as_deref() {
            if args.output.is_some(){return Err("接入命令不接受输出路径；未导入或下载");}
            return connect_explicit_profile(profile,&id,args.expect_title.as_deref()).await;
        }
        let output=args.output.as_deref().ok_or("下载须提供新的绝对.wav保存路径")?;
        output_guard(output)?;
        let auth=AuthState::load().map_err(|_|"没有可用的Sunox既有登录态；请按当前明确授权接入自己的单一配置，未自动登录")?;
        let client=SunoClient::new_for_studio_download(auth).await.map_err(|_|"现有Sunox登录态刷新未成功；未自动登录或继续")?;
        operation(&client,&id,output,args.expect_title.as_deref(),Duration::from_secs(2)).await
    }.await;
    match result {Ok(value)=>{println!("{value}");0},Err(reason)=>{println!("{}",serde_json::json!({"tool_version":env!("CARGO_PKG_VERSION"),"status":"failed","reason":reason}));1}}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read,Write},net::TcpListener,sync::{Arc,Mutex}};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD,Engine};
    fn fixture(mode:&str)->(SunoClient,Arc<Mutex<Vec<String>>>,std::thread::JoinHandle<()>) {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();
        let base=format!("http://{address}");let requests=Arc::new(Mutex::new(Vec::new()));let captured=requests.clone();let mode=mode.to_owned();
        let handle=std::thread::spawn(move||{
            let mut prepares=0;
            for incoming in listener.incoming(){
                let mut stream=incoming.unwrap();let mut bytes=Vec::new();let mut buf=[0u8;4096];
                loop{let count=stream.read(&mut buf).unwrap();if count==0{break;}bytes.extend_from_slice(&buf[..count]);if bytes.windows(4).any(|w|w==b"\r\n\r\n"){break;}}
                let raw=String::from_utf8_lossy(&bytes);let first=raw.lines().next().unwrap().to_owned();captured.lock().unwrap().push(first.clone());
                let (status,body,finish)=if first.contains("/api/billing/info/") {
                    let mut value=serde_json::json!({"credits":8904,"total_credits_left":8904,"monthly_usage":0,"monthly_limit":10000,"is_active":true,"plan":{"name":"Premier","plan_key":"premier"},"models":[],"period":"monthly","renews_on":null,"download_usage":{"current_period_downloads_limit":60,"current_period_downloads_used":60,"additional_download_remaining":0}});
                    if mode=="quota_changed" && prepares>0 { value["download_usage"]["current_period_downloads_used"]=serde_json::json!(61); }
                    if mode=="missing_usage" {value.as_object_mut().unwrap().remove("download_usage");}
                    (200,serde_json::to_vec(&value).unwrap(),prepares>0||mode=="missing_usage")
                }else if first.contains("/api/clip/") {
                    let owner=if mode=="wrong_owner"{"someone-else"}else{"owned-user"};
                    let value=serde_json::json!({"id":"00000000-0000-4000-8000-000000000001","title":"Owned Song","status":"complete","model_name":"chirp","audio_url":null,"video_url":null,"image_url":null,"created_at":"2026-10-05","user_id":owner});
                    (200,serde_json::to_vec(&value).unwrap(),matches!(mode.as_str(),"wrong_owner"|"import_ready"))
                }else if first.contains("/api/studio/clip/"){
                    prepares+=1;
                    if mode=="403"{(403,b"forbidden".to_vec(),true)}
                    else if mode=="processing"&&prepares==1{(200,b"{\"ok\":false,\"status\":\"processing\"}".to_vec(),false)}
                    else {(200,serde_json::to_vec(&serde_json::json!({"ok":true,"status":"ready","download_url":format!("http://{address}/native.wav")})).unwrap(),false)}
                }else if first.contains("/native.wav") {
                    assert!(!raw.to_ascii_lowercase().contains("authorization:"),"media transport must not forward auth");
                    let body=if mode=="bad_wav"{b"<html>placeholder</html>".to_vec()}else{wav()};
                    (200,body,mode=="bad_wav")
                }else{(500,b"unexpected route".to_vec(),true)};
                write!(stream,"HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();stream.write_all(&body).unwrap();
                if finish{break;}
            }
        });
        let claims=URL_SAFE_NO_PAD.encode(b"{\"user_id\":\"owned-user\",\"exp\":4102444800}");
        let auth=AuthState{jwt:Some(format!("e30.{claims}.unused")),..Default::default()};
        (SunoClient::new_for_tests(base,auth).unwrap(),requests,handle)
    }
    fn wav()->Vec<u8>{
        let data=vec![0u8;19200];let mut v=Vec::new();v.extend_from_slice(b"RIFF");v.extend_from_slice(&(36u32+data.len() as u32).to_le_bytes());v.extend_from_slice(b"WAVEfmt ");v.extend_from_slice(&16u32.to_le_bytes());v.extend_from_slice(&1u16.to_le_bytes());v.extend_from_slice(&2u16.to_le_bytes());v.extend_from_slice(&48000u32.to_le_bytes());v.extend_from_slice(&192000u32.to_le_bytes());v.extend_from_slice(&4u16.to_le_bytes());v.extend_from_slice(&16u16.to_le_bytes());v.extend_from_slice(b"data");v.extend_from_slice(&(data.len() as u32).to_le_bytes());v.extend_from_slice(&data);v
    }
    #[tokio::test]
    async fn studio_contract_and_safe_file_batch(){
        for mode in ["ready","processing","403","bad_wav","wrong_owner","quota_changed","missing_usage"]{
            let (client,requests,server)=fixture(mode);let folder=tempfile::tempdir().unwrap();let output=folder.path().join("song.wav");
            let result=operation(&client,"00000000-0000-4000-8000-000000000001",&output,Some("Owned Song"),Duration::from_millis(1)).await;
            assert_eq!(result.is_ok(),matches!(mode,"ready"|"processing"),"{mode}: {result:?}");
            assert_eq!(output.exists(),result.is_ok());server.join().unwrap();
            let calls=requests.lock().unwrap();assert!(calls.iter().all(|line|line.starts_with("GET ")),"no authorize or generation POST");
            let prepares=calls.iter().filter(|line|line.contains("/api/studio/")).collect::<Vec<_>>();
            assert_eq!(prepares.len(),if mode=="processing"{2}else if matches!(mode,"wrong_owner"|"missing_usage"){0}else{1});
            assert!(prepares.iter().all(|line|line.contains("/download?format=wav ")));
            assert!(calls.iter().all(|line|!line.contains("/api/download/authorize")));
            if mode=="ready" {let before=std::fs::read(&output).unwrap();assert!(operation(&client,"00000000-0000-4000-8000-000000000001",&output,None,Duration::ZERO).await.is_err());assert_eq!(std::fs::read(&output).unwrap(),before);}
            assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(),usize::from(output.exists()),"no leaked stage files");
        }
    }
    #[tokio::test]
    async fn imported_auth_stays_ephemeral_until_owner_validation(){
        for mode in ["import_ready","wrong_owner","missing_usage"] {
            let (client,requests,server)=fixture(mode);
            let result=validate_import(&client,"00000000-0000-4000-8000-000000000001",Some("Owned Song")).await;
            assert_eq!(result.is_ok(),mode=="import_ready");server.join().unwrap();
            let calls=requests.lock().unwrap();assert!(calls.iter().all(|line|line.starts_with("GET ")&&!line.contains("/studio/")&&!line.contains("authorize")));
        }
        let candidate=AuthState{jwt:Some(format!("e30.{}.unused",URL_SAFE_NO_PAD.encode(b"{\"user_id\":\"owned-user\"}"))),..Default::default()};
        assert!(existing_auth_guard(None,&candidate).is_ok());assert!(existing_auth_guard(Some(&candidate),&candidate).is_ok());
        assert!(existing_auth_guard(Some(&AuthState::default()),&candidate).is_err());
        let other=AuthState{jwt:Some(format!("e30.{}.unused",URL_SAFE_NO_PAD.encode(b"{\"user_id\":\"other-user\"}"))),..Default::default()};
        assert!(existing_auth_guard(Some(&other),&candidate).is_err());
    }
    #[test]
    fn portable_profile_and_dispatch_guards() {
        assert!(crate::auth::explicit_profile_guard(Path::new("relative")).is_err());
        let folder=tempfile::tempdir().unwrap();
        let real=std::fs::canonicalize(folder.path()).unwrap();
        assert!(crate::auth::explicit_profile_guard(&real).is_ok());
        assert!(crate::auth::explicit_profile_guard(&real.join("../other")).is_err());
        #[cfg(unix)] {
            let link=real.join("link");std::os::unix::fs::symlink(&real,&link).unwrap();
            assert!(crate::auth::explicit_profile_guard(&link).is_err());
        }
        assert!(Args::try_parse_from(["tool","00000000-0000-4000-8000-000000000001","--connect-chrome-profile","relative"]).is_ok());
        assert!(Args::try_parse_from(["tool","--connect-profile-2"]).is_err());
    }
    #[test]
    fn source_and_output_guards(){
        assert!(clip_id("https://suno.com/song/00000000-0000-4000-8000-000000000001").is_ok());assert!(clip_id("https://evil.test/song/00000000-0000-4000-8000-000000000001").is_err());assert!(clip_id("https://suno.com/song/00000000-0000-4000-8000-000000000001?token=x").is_err());assert!(output_guard(Path::new("relative.wav")).is_err());assert!(valid_file_url("https://cdn1.suno.ai/native.wav"));assert!(!valid_file_url("https://user:pass@cdn1.suno.ai/native.wav"));
    }
}
