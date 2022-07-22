use super::dao::IUser;
use super::user::{UserAddress};
use crate::api::ApiResult;
use crate::state::AppState;
use ed25519_dalek::{ed25519, ExpandedSecretKey, PublicKey, SignatureError, Signature, Verifier,  SecretKey};
use {
    ruc::*,
    attohttpc,
    base64,
    serde_json,
};
use hex;
use actix_web::{get, post, web, Responder};
use futures::future::err;
use hex::ToHex;
use ring::hmac::verify;
use ring::test::from_hex;
use serde::Deserialize;
use crate::users::user::QueryAll;


async fn verify_sign(form: &UserAddress, state: &AppState) -> Result<()> {

   let decode = base64::decode_config(&form.anonymous_address, base64::URL_SAFE).unwrap();//from_hex(&form.anonymous_address).unwrap();

    match  PublicKey::from_bytes(&decode)
    {
        Ok(public_key) => {
         /*   let key = "eeWQehLAMlJySIS-nEzaU81DKP1Jn1om30lxLlK82H4=";
            let decode = base64::decode_config(key, base64::URL_SAFE).unwrap();
            let mut secret_key = SecretKey::from_bytes(&decode).unwrap();
            let end_key = ExpandedSecretKey::from(&secret_key);
            let sign2 = end_key.sign(&form.signature_data.as_bytes(), &public_key);
            println!("sign2 = {:?}", hex::encode(sign2));*/

            let sign_hex = from_hex(&form.signature).unwrap();
            let sign = ed25519::Signature::from_bytes(&sign_hex).unwrap();
            return  public_key.verify(&form.signature_data.as_bytes(), &sign).c(d!());
        },
        Err(e) => {
            error!("verify fail");
            return Err(eg!("verify fail{:?}",e));
        }
    }
}

#[get("/address/{address}")]
async fn get_address(address: web::Path<String>, state: AppState) -> impl Responder {
    let address = address.into_inner();
    match state.get_ref().adress_query(&address).await {
        Ok(addex) => {
            ApiResult::new().with_msg("ok").with_data(addex)
        },
        Err(_) => {
            println!("no found address");
            return  ApiResult::new().code(400).with_msg("no found address");
        }
    }
}

#[get("/addressall")]
async fn get_address_all(form: web::Json<QueryAll>, state: AppState) -> impl Responder {
    let form = form.into_inner();
    match state.get_ref().adress_all(form.limit, form.offset).await {
        Ok(addex) => {
            ApiResult::new().with_msg("ok").with_data(addex)
        },
        Err(_) => {
            println!("no found file");
            return  ApiResult::new().code(400).with_msg("no found file");
        }
    }
}

#[post("/address")]
async fn user_address(form: web::Json<UserAddress>, state: AppState) -> impl Responder {
    let form = form.into_inner();
    if let Err(e) = verify_sign(&form, &state).await{
        print!("Signature verification failed, Err = {:?}",e);
        return ApiResult::new().code(400).with_msg("Signature verification failed");
    }
    println!("Signature verification sucess");

    let url = state.get_ref().config.request_rpc.clone() + &*form.transaction_hash.clone();
    let resp = attohttpc::get(&url).send().c(d!()).unwrap();
    if resp.is_success() {
        let object: serde_json::Value = resp.json_utf8().c(d!()).unwrap();
        match object.get("result").c(d!())
        {
            Ok(rpc_result) => {
                let tx = rpc_result.get("tx").c(d!()).unwrap();
                let genesis_str = tx.as_str().unwrap();
              //  println!("Request success : tx = {:?}", std::str::from_utf8(genesis_str.as_ref()).unwrap());
                let tx_decode = base64::decode_config(&genesis_str, base64::URL_SAFE)
                    .map_err(|e| error!("erro : {:?}", e)).unwrap();

                let str_json = String::from_utf8(tx_decode).unwrap();
                let object_tx:serde_json::Value = serde_json::from_str(str_json.as_str()).unwrap();

                if object_tx.is_object()
                {
                    let body_signatures = object_tx.get("body").c(d!()).unwrap_or(&serde_json::Value::Null  )
                        .get("operations").c(d!()).unwrap_or(&serde_json::Value::Null  )
                        .get(0).c(d!()).unwrap_or(&serde_json::Value::Null  )
                        .get("TransferAsset").c(d!()).unwrap_or(&serde_json::Value::Null  )
                        .get("body_signatures").c(d!()).unwrap_or(&serde_json::Value::Null  );

                    if body_signatures.is_array() {
                        let vec_json =  body_signatures.as_array().c(d!()).unwrap();
                        let mut b_find = false;
                        for json_address in vec_json.iter(){


                            let data_address = json_address
                                .get("address").c(d!()).unwrap_or(&serde_json::Value::Null  )
                                .get("key").c(d!()).unwrap_or(&serde_json::Value::Null  );

                            let address_str = data_address.as_str().unwrap();
                            if form.anonymous_address == address_str
                            {
                                b_find = true;
                                println!("tx Address success");
                                break;
                            }
                        }
                        if !b_find
                        {
                            return ApiResult::new().code(400).with_msg("tx Address mismatch");
                        }
                    }
                }
            }
            Err(_) => {
                return ApiResult::new().code(400).with_msg("tx not found");
            }
        }
    } else {
        return ApiResult::new().code(400).with_msg("tx Request failed");
    }

    if !form.share_address.is_empty() && form.share_address != form.evm_anonymous_address{
        let mut b_find = false;
        match state.get_ref().adress_query(&form.share_address.clone()).await {
            Ok(addex) => {
                b_find = true;
                let post_email:String = addex.post_email.parse::<String>().unwrap();
                if post_email.is_empty() && !form.post_email.is_empty(){
                    if let Err(e) = state.get_ref().mail_update(&form.share_address.clone(), &form.post_email.clone()).await {
                        return  ApiResult::new().code(400).with_msg(e.to_string());
                    }
                }

                let mut addres :i16 = addex.experience.parse::<i16>().unwrap();
                addres = addres + 1;
                match state.get_ref().adress_update(&form.share_address.clone(), &addres.to_string()).await {
                    Ok(_) => {}
                    Err(e) => {
                      return  ApiResult::new().code(400).with_msg(e.to_string());
                    }
                }
            },
            Err(_) => {
                print!("Not Find Address");
            }
        }
        if !b_find{
            match state.get_ref().adress_add(&form.share_address, "1", &form.post_email).await {
                Ok(_) => {}
                Err(e) => {
                    return ApiResult::new().code(400).with_msg(e.to_string());
                }
            }
        }
    }

    match state.get_ref().adress_query(&form.evm_anonymous_address.clone()).await {
        Ok(addex) => {
            let post_email:String = addex.post_email.parse::<String>().unwrap();
            if post_email.is_empty() && !form.post_email.is_empty(){
                if let Err(e) = state.get_ref().mail_update(&form.evm_anonymous_address.clone(), &form.post_email.clone()).await {
                    return  ApiResult::new().code(400).with_msg(e.to_string());
                }
            }

            let mut experience :i16 = addex.experience.parse::<i16>().unwrap();
            experience = experience + 1;
            match state.get_ref().adress_update(&form.evm_anonymous_address.clone(), &experience.to_string()).await {
                Ok(res) => {
                   return ApiResult::new().with_msg("ok").with_data(res);
                }
                Err(e) => {
                    return  ApiResult::new().code(400).with_msg(e.to_string());
                }
            }
        },
        Err(_) => {
            print!("no find address");
        }
    }
    match state.get_ref().adress_add(&form.evm_anonymous_address, "1", &form.post_email).await {
        Ok(res) => {
            ApiResult::new().with_msg("ok").with_data(res)
        }
        Err(e) => {
            ApiResult::new().code(400).with_msg(e.to_string())
        }
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(user_address);
    cfg.service(get_address);
    cfg.service(get_address_all);
}
